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
            # XCreateFontCursor would open libXcursor with dlopen, which a
            # static binary does not have.
            configureFlags = (old.configureFlags or [ ]) ++ [ "--disable-loadable-xcursor" ];
            postConfigure = (old.postConfigure or "") + ''
              grep -rlE "$out/(share|lib)/X11" --include='*.h' . \
                | xargs sed -i -e "s|$out/share/X11|/usr/share/X11|g" \
                  -e "s|$out/lib/X11/locale|/usr/share/X11/locale|g"
              # The install of XCMS.txt goes to $(X11dir), so only the define
              # changes.
              sed -i 's|-DXCMSDIR=\\"$(X11dir)\\"|-DXCMSDIR=\\"/usr/share/X11\\"|' src/xcms/Makefile
              grep -q 'DXCMSDIR=\\"/usr/share/X11' src/xcms/Makefile
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
            mkdir -p $out/lib $out/share/licenses
            for p in ${lib.concatMapStringsSep " " (p: "${lib.getLib p}") libs}; do
              cp -n $p/lib/*.a $out/lib/
            done
            # The builtins of gcc that the libraries call, such as
            # __popcountdi2, from the compiler that built them.
            cp ${s.stdenv.cc.cc}/lib/gcc/*/*/libgcc.a $out/lib/
            # The build scripts of wayland-sys and tiny-xlib ask pkg-config
            # for these. The files name the directory relative to their own,
            # so they still work after a copy of $out.
            mkdir $out/lib/pkgconfig
            for pc in x11:X11 x11-xcb:X11-xcb wayland-client:wayland-client; do
              cat > $out/lib/pkgconfig/''${pc%%:*}.pc <<EOF
            libdir=\''${pcfiledir}/..
            Name: ''${pc%%:*}
            Description: ''${pc%%:*}
            Version: 0
            Libs: -L\''${libdir} -l''${pc#*:}
            EOF
            done
            # The licenses ask for their notices in every copy of the binary.
            ${lib.concatMapStringsSep "\n" (p: ''
              dir=$out/share/licenses/${p.pname}
              mkdir -p $dir
              if [ -d ${p.src} ]; then src=${p.src}; else mkdir src-${p.pname}; tar xf ${p.src} -C src-${p.pname}; src=$(echo src-${p.pname}/*); fi
              for name in COPYING LICENSE LICENSE.md; do
                if [ -f $src/$name ]; then cp $src/$name $dir/; fi
              done
              [ -n "$(ls $dir)" ] || { echo "no license in ${p.pname}"; exit 1; }
            '') libs}
          '';
        });
    };
}
