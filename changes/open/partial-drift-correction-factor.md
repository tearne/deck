# Partial Drift Correction Factor

## Intent

The partial drift correction (mapped in the Partial Drift Correction node) pulls each frame's rendered position 0.2% toward the audio's true position, acting as a low-pass filter on audio-batch step noise. Now that the user has a clearer mental model of the rendering pipeline, they're curious whether adjusting the 0.002 factor — up or down — yields visibly smoother scrolling.
