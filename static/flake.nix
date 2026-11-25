{
  description = "The static musl libraries that winit and softbuffer open with dlopen, for a static sgleam on Linux";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/b51242d7d43689db2f3be91bd05d5b24fbb469c4";

  outputs = { self, nixpkgs }:
    let
      forSystems = f: nixpkgs.lib.genAttrs [ "x86_64-linux" "aarch64-linux" ] (system:
        f (import nixpkgs { inherit system; }));
    in
    {
      packages = forSystems (pkgs:
        let
          s = pkgs.pkgsStatic;
          x = s.xorg;
          libs = [
            x.libX11 x.libxcb x.libXau x.libXdmcp x.libXcursor x.libXrender
            x.libXfixes x.libXi x.libXext s.libxkbcommon s.wayland s.libffi
          ];
        in
        {
          default = pkgs.runCommand "sgleam-static-libs" { } ''
            mkdir -p $out/lib
            for p in ${pkgs.lib.concatMapStringsSep " " (p: "${pkgs.lib.getLib p}") libs}; do
              cp -n $p/lib/*.a $out/lib/ 2>/dev/null || true
            done
            ls $out/lib
          '';
        });
    };
}
