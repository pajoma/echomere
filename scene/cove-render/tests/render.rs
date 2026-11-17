//! Rendering checks. They run on any adapter, including a software one (lavapipe) without a GPU.

use cove_render::{Renderer, WhaleModel};
use cove_scene::model::{CoveSeed, Echo, Visitor, WhaleLook, WhaleSpecies};
use cove_scene::scene::{CoveScene, SceneFrame};
use cove_scene::{kelp, sky};
use std::path::Path;
use std::sync::{Mutex, MutexGuard, OnceLock};

#[test]
fn renders_on_any_adapter() {
    assert!(Renderer::new_headless().is_ok());
}

#[test]
fn refuses_srgb_outputs() {
    // `finish()` gamma-encodes itself; an sRGB output would encode a second time.
    let r = renderer();
    let made = Renderer::new(
        r.instance().clone(),
        r.adapter().clone(),
        r.device().clone(),
        r.queue().clone(),
        wgpu::TextureFormat::Bgra8UnormSrgb,
    );
    assert!(matches!(made, Err(cove_render::RenderError::SrgbOutput(_))));
}

#[test]
// The owner wants the scene meditative (2026-11-16): a calm sea with a low, long swell, not the rough open
// sea of the Swift reference (0.5 to 2 m).
fn sea_is_calm() {
    let s = renderer().sea_statistics(123.0).unwrap();
    let total = s
        .iter()
        .map(|c| c.height_rms * c.height_rms)
        .sum::<f64>()
        .sqrt();
    assert!(total > 0.2 && total < 0.5, "height RMS {total}");
}

// The rendering and lighting checks, ported from `SceneChecks/main.swift` (the "Rendering" block) and the
// lighting plan's Task 1. Every snapshot is 320 x 200.

const NOON: f64 = 1_794_000_000.0;
const HOUR: f64 = 3600.0;
const DAY: f64 = 24.0 * HOUR;
const W: u32 = 320;
const H: u32 = 200;

fn visitor(id: &str, arrived: f64, species: WhaleSpecies) -> Visitor {
    Visitor {
        creature_id: id.to_string(),
        look: WhaleLook::new(species, 1.0),
        arrived_at: arrived,
    }
}

fn echo(id: &str, left: f64) -> Echo {
    Echo {
        creature_id: id.to_string(),
        look: WhaleLook::new(WhaleSpecies::SouthernRightWhale, 1.0),
        left_at: left,
    }
}

/// The frame every Swift rendering check draws: Munich bay at noon, with guests and echoes.
fn frame() -> SceneFrame {
    let guests = [
        visitor(
            "whale-a",
            NOON - 5.0 * HOUR,
            WhaleSpecies::SouthernRightWhale,
        ),
        visitor("whale-b", NOON - 2.0 * HOUR, WhaleSpecies::Orca),
        visitor("whale-c", NOON - HOUR, WhaleSpecies::SouthernRightWhale),
    ];
    let traces = [
        echo("whale-x", NOON - DAY),
        echo("whale-y", NOON - 3.0 * DAY),
    ];
    CoveScene::new(CoveSeed(4_711)).frame(NOON, &guests, &traces)
}

/// The same frame under the sky of another hour.
fn at_hour(h: f64) -> SceneFrame {
    let mut f = frame();
    f.sky = sky::state(h);
    f
}

/// One renderer for every pixel check. Opening many devices at once on lavapipe crashes now and then
/// (SIGSEGV in about one parallel run in ten), and one device is faster too.
fn renderer() -> MutexGuard<'static, Renderer> {
    static RENDERER: OnceLock<Mutex<Renderer>> = OnceLock::new();
    RENDERER
        .get_or_init(|| Mutex::new(Renderer::new_headless().unwrap()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn snapshot(r: &mut Renderer, f: &SceneFrame) -> image::RgbaImage {
    r.snapshot(f, W, H).unwrap()
}

/// Pixels as RGB in 0..1, row-major from the top.
fn rgb(image: &image::RgbaImage) -> Vec<[f64; 3]> {
    image
        .pixels()
        .map(|p| {
            [
                p[0] as f64 / 255.0,
                p[1] as f64 / 255.0,
                p[2] as f64 / 255.0,
            ]
        })
        .collect()
}

fn mean_luma(pixels: &[[f64; 3]]) -> f64 {
    pixels
        .iter()
        .map(|c| 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2])
        .sum::<f64>()
        / pixels.len() as f64
}

fn luma_at(r: &mut Renderer, h: f64) -> f64 {
    mean_luma(&rgb(&snapshot(r, &at_hour(h))))
}

#[test]
fn same_frame_same_pixels() {
    let mut r = renderer();
    let f = frame();
    assert!(snapshot(&mut r, &f) == snapshot(&mut r, &f));
}

#[test]
fn neither_black_nor_washed_out() {
    let mut r = renderer();
    let luma = mean_luma(&rgb(&snapshot(&mut r, &frame())));
    eprintln!("mean luma {luma:.4}");
    assert!(luma > 0.05 && luma < 0.35, "mean luma {luma}");
}

#[test]
fn noon_brighter_than_midnight() {
    let mut r = renderer();
    let (noon, midnight) = (luma_at(&mut r, 12.0), luma_at(&mut r, 0.0));
    eprintln!("luma noon {noon:.4}, midnight {midnight:.4}");
    assert!(noon > 1.5 * midnight, "noon {noon}, midnight {midnight}");
}

#[test]
fn dark_but_not_black_at_0_6_18() {
    let mut r = renderer();
    for h in [0.0, 6.0, 18.0] {
        let luma = luma_at(&mut r, h);
        eprintln!("luma at {h}:00 {luma:.4}");
        assert!(luma > 0.02, "luma at {h}:00 is {luma}");
    }
}

#[test]
fn golden_hour_not_washed_out() {
    let mut r = renderer();
    let luma = luma_at(&mut r, 7.0);
    eprintln!("luma at 7:00 {luma:.4}");
    assert!(luma < 0.35, "luma at 7:00 is {luma}");
}

#[test]
fn deep_is_blue() {
    let mut r = renderer();
    let pixels = rgb(&snapshot(&mut r, &at_hour(12.0)));
    // The bottom fifth of the picture looks into the deep.
    let deep = &pixels[(W * H * 4 / 5) as usize..];
    let mut avg = [0.0; 3];
    for c in deep {
        for i in 0..3 {
            avg[i] += c[i] / deep.len() as f64;
        }
    }
    eprintln!("deep mean rgb {avg:.4?}");
    assert!(avg[2] > avg[1] && avg[1] > avg[0], "deep mean rgb {avg:?}");
}

#[test]
fn no_black_or_nan_pixels_at_noon() {
    let mut r = renderer();
    let pixels = rgb(&snapshot(&mut r, &at_hour(12.0)));
    let darkest = pixels
        .iter()
        .map(|c| c[0] + c[1] + c[2])
        .fold(f64::MAX, f64::min);
    eprintln!("darkest r+g+b {darkest:.4}");
    assert!(darkest > 1.0 / 255.0, "darkest pixel r+g+b {darkest}");
}

#[test]
fn shader_time_wraps_daily() {
    let mut r = renderer();
    let f = frame();
    // The same moment of the day 400 days later. Only the time changes, so whatever else the scene would
    // do over 400 days (echoes fading, whales leaving) cannot hide what the shaders do with the time.
    let mut later = f.clone();
    later.time += 400.0 * DAY;
    assert!(snapshot(&mut r, &f) == snapshot(&mut r, &later));
}

#[test]
fn kelp_is_drawn() {
    let mut r = renderer();
    let with = frame();
    assert!(!with.kelp.is_empty(), "the test frame has kelp");
    let mut without = with.clone();
    without.kelp.clear();
    let (a, b) = (snapshot(&mut r, &with), snapshot(&mut r, &without));
    let differing = a
        .pixels()
        .zip(b.pixels())
        .filter(|(p, q)| (0..3).any(|i| (p[i] as i32 - q[i] as i32).abs() > 2))
        .count();
    let share = differing as f64 / (W * H) as f64;
    eprintln!("kelp changes {:.2} % of the pixels", share * 100.0);
    assert!(
        share >= 0.001,
        "kelp changes only {:.3} % of the pixels",
        share * 100.0
    );
}

/// The share of pixels in which two snapshots differ by more than 2 of 255 in some channel.
fn differing_share(a: &image::RgbaImage, b: &image::RgbaImage) -> f64 {
    let differing = a
        .pixels()
        .zip(b.pixels())
        .filter(|(p, q)| (0..3).any(|i| (p[i] as i32 - q[i] as i32).abs() > 2))
        .count();
    differing as f64 / (a.width() * a.height()) as f64
}

#[test]
fn missing_whale_file_is_an_error() {
    let r = renderer();
    let err = WhaleModel::load(
        Path::new("/nonexistent"),
        WhaleSpecies::Orca,
        r.device(),
        r.queue(),
    )
    .unwrap_err();
    assert!(err.to_string().contains("orca.mesh"), "{err}");
}

/// 75 minutes after the test frame's noon two of the three guests are in view (checked with
/// `cove_scene::camera::project` at aspect 1.6): the orca 38 m away, left of centre and high, and a
/// southern right whale 69 m away. Under the noon sky.
fn whales_in_view() -> SceneFrame {
    let guests = [
        visitor(
            "whale-a",
            NOON - 5.0 * HOUR,
            WhaleSpecies::SouthernRightWhale,
        ),
        visitor("whale-b", NOON - 2.0 * HOUR, WhaleSpecies::Orca),
        visitor("whale-c", NOON - HOUR, WhaleSpecies::SouthernRightWhale),
    ];
    let mut f = CoveScene::new(CoveSeed(4_711)).frame(NOON + 4_500.0, &guests, &[]);
    f.sky = sky::state(12.0);
    f
}

#[test]
fn whales_are_drawn() {
    let mut r = renderer();
    let with = whales_in_view();
    assert_eq!(with.whales.len(), 3, "the test frame has three whales");
    // Only the whales are taken out, so nothing else that the visitors might change can pass the check.
    let mut without = with.clone();
    without.whales.clear();
    let share = differing_share(&snapshot(&mut r, &with), &snapshot(&mut r, &without));
    eprintln!("whales change {:.2} % of the pixels", share * 100.0);
    assert!(
        share >= 0.001,
        "whales change only {:.3} % of the pixels",
        share * 100.0
    );
}

#[test]
fn fish_are_drawn() {
    let mut r = renderer();
    let with = frame();
    assert!(!with.creatures.is_empty(), "the test frame has fish");
    let mut without = with.clone();
    without.creatures.clear();
    let share = differing_share(&snapshot(&mut r, &with), &snapshot(&mut r, &without));
    eprintln!("fish change {:.2} % of the pixels", share * 100.0);
    assert!(
        share >= 0.0005,
        "fish change only {:.3} % of the pixels",
        share * 100.0
    );
}

/// The pixels of `with` that differ from `without`, as RGB in 0..1.
fn differing_pixels(with: &image::RgbaImage, without: &image::RgbaImage) -> Vec<[f64; 3]> {
    with.pixels()
        .zip(without.pixels())
        .filter(|(a, b)| a != b)
        .map(|(a, _)| {
            [
                a[0] as f64 / 255.0,
                a[1] as f64 / 255.0,
                a[2] as f64 / 255.0,
            ]
        })
        .collect()
}

#[test]
fn night_fish_are_not_red() {
    // Red is absorbed first in water: at night the fish must not turn red-brown against the blue.
    // The cove of the look snapshots (seed 7), whose near school is large on screen; the fish of the
    // seed-4711 test frame are too far away to show their colour.
    let mut r = renderer();
    for h in [21.0, 0.0] {
        let mut with = CoveScene::new(CoveSeed(7)).frame(1_794_000_500.0, &[], &[]);
        with.sky = sky::state(h);
        let mut without = with.clone();
        without.creatures.clear();
        // Large enough that the near fish have bodies, not only edges blended with the water.
        let fish = differing_pixels(
            &r.snapshot(&with, 960, 600).unwrap(),
            &r.snapshot(&without, 960, 600).unwrap(),
        );
        assert!(!fish.is_empty(), "fish are drawn at {h}:00");
        // The fish's bodies: the darkest tenth of the changed pixels.
        let mut body = fish.clone();
        body.sort_by(|a, b| mean_luma(&[*a]).partial_cmp(&mean_luma(&[*b])).unwrap());
        body.truncate(body.len() / 10);
        let mean = |i: usize| body.iter().map(|c| c[i]).sum::<f64>() / body.len() as f64;
        let red = body.iter().filter(|c| c[0] > c[2]).count() as f64 / body.len() as f64;
        eprintln!(
            "fish bodies at {h}:00: mean r {:.4}, b {:.4}, {:.1} % redder than blue",
            mean(0),
            mean(2),
            red * 100.0
        );
        assert!(
            mean(2) >= mean(0),
            "fish at {h}:00 are redder than blue: r {:.4} b {:.4}",
            mean(0),
            mean(2)
        );
        assert!(
            red < 0.01,
            "{:.1} % of the fish pixels at {h}:00 are redder than blue",
            red * 100.0
        );
    }
}

/// The luma below which `share` of the pixels lie.
fn luma_percentile(image: &image::RgbaImage, share: f64) -> f64 {
    let mut l: Vec<f64> = rgb(image)
        .iter()
        .map(|c| 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2])
        .collect();
    l.sort_by(|a, b| a.partial_cmp(b).unwrap());
    l[((l.len() - 1) as f64 * share) as usize]
}

#[test]
fn moonlight_is_no_spotlight() {
    // The moon is far dimmer than the sun: the brightest 0.1 % of a moonlit frame (shafts, the moon's
    // glint) stay well below the brightest 0.1 % at noon.
    let mut r = renderer();
    let noon = luma_percentile(&snapshot(&mut r, &at_hour(12.0)), 0.999);
    let morning = luma_percentile(&snapshot(&mut r, &at_hour(7.0)), 0.999);
    let night = luma_percentile(&snapshot(&mut r, &at_hour(21.0)), 0.999);
    eprintln!("99.9th percentile luma: 21h {night:.3}, 7h {morning:.3}, 12h {noon:.3}");
    assert!(
        night < 0.6 * noon,
        "moonlit peak {night:.3} vs noon peak {noon:.3}"
    );
}

// The owner (2026-11-16): "at the bottom it should be really dark". The lowest fifth of the picture looks
// out sideways into the deep, the top fifth up towards the surface.
#[test]
fn bottom_is_really_dark() {
    let mut r = renderer();
    let pixels = rgb(&snapshot(&mut r, &at_hour(12.0)));
    let rows = H as usize / 5;
    let top = mean_luma(&pixels[..rows * W as usize]);
    let bottom = mean_luma(&pixels[pixels.len() - rows * W as usize..]);
    assert!(
        bottom < 0.1 && bottom < 0.4 * top,
        "bottom {bottom}, top {top}"
    );
}

fn mean_rgb(pixels: &[[f64; 3]]) -> [f64; 3] {
    let n = pixels.len() as f64;
    let sum = pixels
        .iter()
        .fold([0.0; 3], |s, c| [s[0] + c[0], s[1] + c[1], s[2] + c[2]]);
    [sum[0] / n, sum[1] / n, sum[2] / n]
}

// The owner (2026-11-16): sunrise and sunset colour the water warm; the day stays blue.
#[test]
fn sunset_is_warm_noon_is_blue() {
    let mut r = renderer();
    let dusk = mean_rgb(&rgb(&snapshot(&mut r, &at_hour(17.0))));
    let noon = mean_rgb(&rgb(&snapshot(&mut r, &at_hour(12.0))));
    assert!(dusk[0] > dusk[2], "17:00 mean rgb {dusk:?}");
    assert!(noon[0] < noon[2], "noon mean rgb {noon:?}");
}

// The kelp frames the picture: the pixels it changes lie at the left and right edges, on screens of every
// shape and at every moment of the swell and the sway (kelp design, 2026-11-16).
#[test]
fn kelp_keeps_the_middle_free() {
    let mut r = renderer();
    for (w, h) in [(320u32, 200u32), (320, 240), (420, 180), (180, 320)] {
        // Eight moments over 175 s: the swell's push (44 s) and the blades' slower sway (~50 s and more).
        for i in 0..8 {
            // In daylight, where the kelp stands out against the water (the test frame's own time is night).
            let mut with = at_hour(12.0);
            with.kelp_motion = kelp::motion(with.time + i as f64 * 25.0);
            let mut without = with.clone();
            without.kelp.clear();
            let (a, b) = (
                r.snapshot(&with, w, h).unwrap(),
                r.snapshot(&without, w, h).unwrap(),
            );
            let band = (w as f64 * 0.2).floor() as u32;
            let (mut left, mut right, mut middle) = (0usize, 0usize, 0usize);
            for y in 0..h {
                for x in 0..w {
                    let (p, q) = (a.get_pixel(x, y), b.get_pixel(x, y));
                    if (0..3).any(|c| (p[c] as i32 - q[c] as i32).abs() > 2) {
                        if x < band {
                            left += 1;
                        } else if x >= w - band {
                            right += 1;
                        } else {
                            middle += 1;
                        }
                    }
                }
            }
            let total = (left + right + middle) as f64;
            let band_pixels = (band * h) as f64;
            assert!(
                middle as f64 <= 0.05 * total,
                "{w}x{h} #{i}: {middle} of {total} changed pixels in the middle"
            );
            assert!(
                left as f64 >= 0.02 * band_pixels && right as f64 >= 0.02 * band_pixels,
                "{w}x{h} #{i}: left {left}, right {right} of {band_pixels} per band"
            );
        }
    }
}
