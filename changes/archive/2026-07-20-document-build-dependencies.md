# Document build dependencies

**Mode:** Wander

## Intent

The README's Installation section only lists ALSA/PipeWire as a runtime dependency. It doesn't mention what's needed to actually compile `deck` from source: pkg-config plus the ALSA development headers (`libasound2-dev` on Debian/Ubuntu — the plain `alsa` package does not include this), and a C++ compiler, needed because the `soundtouch` crate compiles a bundled C++ library via `cc`.

This just caused a real build failure: `alsa` was installed but not `libasound2-dev`, and `pkg-config --libs --cflags alsa` failed until the dev package was added. Anyone building from source on a fresh machine is likely to hit the same wall.

Add a build-dependencies note to the README so this is documented before it bites someone else.

## Conclusion

Added a "Build dependencies (Linux)" bullet list to the README's Installation section, plus a copyable `apt install pkg-config libasound2-dev g++` line ahead of the `cargo build` step. Initial pass was prose; revised to bullets + copyable command on request, which reads better for a quick-reference dependency list. No other docs affected.
