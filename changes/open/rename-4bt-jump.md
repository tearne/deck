# Rename jump_forward_4bt / jump_backward_4bt

## Intent

The config actions `jump_forward_4bt` and `jump_backward_4bt` describe the jump as "4 beats" but 4 beats = 1 bar, and the in-app overlay already labels key 2 as `+1b` (1 bar). Rename to `jump_forward_1b` / `jump_backward_1b` so the action names match the overlay labels and the bar-unit convention used by all the larger jumps.
