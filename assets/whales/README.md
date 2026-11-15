# Whale model sources

The original whale models are not in this repository (they are large). To rebuild the files the app uses,
download each model from Sketchfab as GLB, put it in `assets/whales/source/` under the file name below, and run
`python3 tools/prepare-whales.py`.

| File name | Model | Licence |
|---|---|---|
| `model_55a_-_southern_right_whale.glb` | [Model 55A - Southern Right Whale](https://sketchfab.com/3d-models/model-55a-southern-right-whale-3bddfdfafc6c43758df9b94bc9e0fe9e), DigitalLife3D | CC BY-NC 4.0 |
| `female_orca.glb` | [Female Orca](https://sketchfab.com/3d-models/female-orca-e5eb46758cd242efabe49954cdd16980), neurodolphin | CC BY-NC 4.0 |

The prepared files and their licence (`LICENSE.md`, CC BY-NC 4.0) are in `assets/whales/prepared/`. Every
platform's renderer reads them from there: the Rust renderer looks for the `assets` folder in
`$ECHOMERE_ASSETS`, next to its executable, or in the repository. The Swift package still has its own copy in
`macos/EchomereScene/Sources/CoveSceneRenderer/Resources/Whales/` until it is retired;
`tools/prepare-whales.py` refreshes both.
