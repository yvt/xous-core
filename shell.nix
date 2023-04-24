with import <nixpkgs> {};

with pkgs.xorg;

runCommand "dummy" {
  nativeBuildInputs = [
    rustup
  ];

  buildInputs = [
    alsaLib pkg-config
  ];

  LD_LIBRARY_PATH = [
    ("${wayland}/lib:"
    + "${libX11}/lib:${libXcursor}/lib:${libXrandr}/lib:"
    + "${libXi}/lib:${libXext}/lib:${libGL}/lib")
  ];
} ""

