pkg config:

sh/bash/zsh:
```sh
export PKG_CONFIG_PATH="$(brew --prefix libkrun)/lib/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
pkg-config --modversion libkrun
pkg-config --variable=includedir libkrun
pkg-config --variable=libdir libkrun
pkg-config --cflags --libs libkrun
```
