{
  description = "noctua-colors — a generated colour system: CSS custom properties, a Tailwind v4 theme, DTCG 2025.10 tokens, SCSS, JSON/TypeScript, QML, and a const Rust crate";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];

      # Plain `genAttrs` rather than flake-utils. It is one function against a
      # whole extra input, and the community moved away from flake-utils for
      # exactly this shape of flake.
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});

      version = (builtins.fromJSON (builtins.readFile ./package.json)).version;
    in
    {
      packages = forAllSystems (pkgs: rec {
        noctua-colors = pkgs.stdenvNoCC.mkDerivation {
          pname = "noctua-colors";
          inherit version;

          # Only the generated artifacts. The compiler is not what a consumer
          # wants from Nix — the output is.
          src = ./dist;

          # `src` is a directory, not an archive, and there is nothing to
          # configure or build: these files were compiled from the spec before
          # they were committed.
          dontUnpack = true;
          dontConfigure = true;
          dontBuild = true;

          installPhase = ''
            runHook preInstall
            mkdir -p "$out/share/noctua-colors"
            cp -r "$src"/. "$out/share/noctua-colors/"
            runHook postInstall
          '';

          meta = with pkgs.lib; {
            description = "Generated colour system: CSS, DTCG tokens, Tailwind v4 theme, SCSS, TS, QML";
            longDescription = ''
              Every artifact noctua-colors emits, installed under
              share/noctua-colors. Reference it from another derivation:

                installPhase = '''
                  mkdir -p $out/static
                  cp -r ''${noctua-colors}/share/noctua-colors/css $out/static/
                ''';

              A consumer who does not want this flake as an input can instead
              fetchurl the release tarball, whose assets carry signed build
              provenance.
            '';
            homepage = "https://noctua-world.github.io/noctua-colors/";
            downloadPage = "https://github.com/noctua-world/noctua-colors/releases";
            license = with licenses; [ mit asl20 ];
            platforms = platforms.all;
          };
        };

        default = noctua-colors;
      });

      # So a consumer can add the overlay and reach `pkgs.noctua-colors`
      # alongside everything else, rather than threading a flake output through
      # their module arguments.
      overlays.default = final: _prev: {
        noctua-colors = self.packages.${final.stdenv.hostPlatform.system}.noctua-colors;
      };
    };
}
