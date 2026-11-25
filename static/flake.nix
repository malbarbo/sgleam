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
          lib = pkgs.lib;

          # The libraries read data at run time from paths that the build
          # writes into them. In nixpkgs these paths are in /nix/store, which
          # the machine of a student does not have, so they point to the
          # places that the other distributions use.
          libx11 = s.libx11.overrideAttrs (old: {
            postConfigure = (old.postConfigure or "") + ''
              grep -rlE "$out/(share|lib)/X11" --include='*.h' --include=Makefile . \
                | xargs sed -i -e "s|$out/share/X11|/usr/share/X11|g" \
                  -e "s|$out/lib/X11/locale|/usr/share/X11/locale|g"
            '';
          });
          libxkbcommon = s.libxkbcommon.overrideAttrs (old: {
            mesonFlags = lib.filter
              (flag: !(lib.hasPrefix "-Dxkb-config-root=" flag || lib.hasPrefix "-Dx-locale-root=" flag))
              old.mesonFlags ++ [
              "-Dxkb-config-root=/usr/share/X11/xkb"
              "-Dxkb-config-versioned-extensions-path=/usr/share/xkeyboard-config-2.d"
              "-Dxkb-config-unversioned-extensions-path=/usr/share/xkeyboard-config.d"
              "-Dx-locale-root=/usr/share/X11/locale"
            ];
            # The legacy root comes from the xkeyboard-config of nixpkgs, and
            # no option of meson sets it.
            postConfigure = (old.postConfigure or "") + ''
              sed -i -E 's|/nix/store/[^"]*/share/X11/xkb|/usr/share/X11/xkb|g' config.h
            '';
          });
          libxcursor = s.libxcursor.overrideAttrs (old: {
            configureFlags = (old.configureFlags or [ ]) ++ [
              "--with-icondir=/usr/share/icons"
              "--with-cursorpath=~/.local/share/icons:~/.icons:/usr/share/icons:/usr/share/pixmaps"
            ];
          });

          libs = [
            libx11 s.libxcb s.libxau s.libxdmcp libxcursor s.libxrender
            s.libxfixes s.libxi s.libxext libxkbcommon s.wayland s.libffi
          ];
        in
        {
          default = pkgs.runCommand "sgleam-static-libs" { } ''
            mkdir -p $out/lib
            for p in ${lib.concatMapStringsSep " " (p: "${lib.getLib p}") libs}; do
              cp -n $p/lib/*.a $out/lib/
            done
            # The builtins of gcc that the libraries call, such as
            # __popcountdi2, from the compiler that built them.
            cp ${s.stdenv.cc.cc}/lib/gcc/*/*/libgcc.a $out/lib/
            ls $out/lib
          '';
        });
    };
}
