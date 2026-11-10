# Licence of the whale models

The files in this folder are **not** under Echomere's MIT licence. They are adapted from the models below and
are licensed under [Creative Commons Attribution-NonCommercial 4.0 International (CC BY-NC 4.0)](https://creativecommons.org/licenses/by-nc/4.0/).

That means: you may copy, share and adapt them, as long as you give credit as below and do **not** use them
for commercial purposes. Any build of Echomere that contains these files may not be sold.

## southern-right-whale.mesh, southern-right-whale.jpg

- Adapted from "Model 55A - Southern Right Whale" by DigitalLife3D
- Source: https://sketchfab.com/3d-models/model-55a-southern-right-whale-3bddfdfafc6c43758df9b94bc9e0fe9e
- Author: https://sketchfab.com/DigitalLife3D (licensing enquiries: https://www.digitallife3d.org)
- Licence: CC BY-NC 4.0

## orca.mesh, orca.jpg

- Adapted from "Female Orca" by neurodolphin
- Source: https://sketchfab.com/3d-models/female-orca-e5eb46758cd242efabe49954cdd16980
- Author: https://sketchfab.com/labmonkey
- Licence: CC BY-NC 4.0

## Changes made

Made with `tools/prepare-whales.py` from the original GLB files:

- only the body mesh is kept (for the orca, also the eyeballs); skeleton, animations, teeth, tongue, baleen,
  normal maps and other textures are left out;
- the mesh is turned to a common orientation and scaled to a length of one unit;
- the colour texture is reduced from 4096 to 1024 pixels and saved as JPEG.

The app animates the whales itself.
