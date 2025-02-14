{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    systems.url = "github:nix-systems/default";
    devenv.url = "github:cachix/devenv";
  };

  nixConfig = {
    extra-trusted-public-keys = "devenv.cachix.org-1:w1cLUi8dv3hnoSPGAuibQv+f9TZLr6cv/Hm9XgU50cw=";
    extra-substituters = "https://devenv.cachix.org";
  };

  outputs = { self, nixpkgs, devenv, systems, ... } @ inputs:
    let
      forEachSystem = nixpkgs.lib.genAttrs (import systems);
    in
    {
      devShells = forEachSystem
        (system:
          let
            pkgs = nixpkgs.legacyPackages.${system};
          in
          {
            default = devenv.lib.mkShell {
              inherit inputs pkgs;
              modules = [
                {
                  env = {
                    RUSTC_WRAPPER = "${pkgs.sccache}/bin/sccache";
                  };
                  languages.rust = {
                    enable = true;
                    components = [ "rustc" "cargo" "clippy" "rustfmt" "rust-analyzer" ];
                  };
                  pre-commit.hooks = {
                    rustfmt.enable = true;
                    clippy.enable = true;
                  };
                  packages = with pkgs; [
                    graphene
                    gtk4
                    glib
                    pango
                    cairo
                    gdk-pixbuf
                    harfbuzz
                    libinput udev
                    cargo-watch
                    cargo-nextest
                    mold
                    sccache
                    libiio
                    xorg.libXrandr
                    xorg.libX11
                  ];
                }
              ];
            };
          });
    };
}
