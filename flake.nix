{
  description = "A simple DNS proxy server written in rust. Macchiato DNS contains some powerful features that can be used to secure your communications.";

  inputs = {
    nixpkgs.url = "nixpkgs/nixos-24.05";
    nix-std.url = "github:chessai/nix-std";
  };

  outputs = {self, nixpkgs, nix-std, ...}: let
    system = "x86_64-linux";
    pname = "macchiato-dns";
    std = nix-std.lib;
  in {
    packages.${system} = let
      pkgs = nixpkgs.legacyPackages.${system};
    in {
      default = pkgs.rustPlatform.buildRustPackage {
        name = pname;
        version = "0.0.1";

        src = ./.;

        doCheck = false;

        cargoLock.lockFile = ./Cargo.lock;

        postInstall = ''
          install ./1hosts_pro.txt $out/1hosts_pro.txt
          cp -r ./static/ $out/static/
        '';
      };
    };
    nixosModules.default = { config , lib , pkgs , ...  }: with lib; let
      cfg = config.services.macchiato-dns;
      configFile = if cfg.configurationPath != null then cfg.configurationPath else builtins.toFile "config.toml"
        (std.serde.toTOML {
          allowed_domains = cfg.allowedDomains;
          auto_update = cfg.autoUpdate;
          external = cfg.external;
          filters_path = if cfg.filtersPath != null then cfg.filtersPath else (toString cfg.package);
          filter_version = cfg.filterVersion;
          overrides = cfg.overrides;
          small = cfg.small;
          verbosity = cfg.verbosity;
          web_password = cfg.webPassword;
        });
    in {
      options.services.macchiato-dns = {
        enable = lib.mkEnableOption "enable the macchiato-dns service";

        package = mkOption {
          type = types.package;
          default = self.packages.x86_64-linux.default;
          description = "macchiato-dns package to use";
        };

        openFirewall = lib.mkOption {
          type = types.bool;
          default = false;
          description = "Open port UDP 53 in the firewall";
        };

        openWebFirewall = lib.mkOption {
          type = types.bool;
          default = false;
          description = "Open port TCP 5554 in the firewall";
        };

        configurationPath = lib.mkOption {
          type = types.nullOr types.str;
          default = null;
          description = "Path to the configuration file. If is set, it overrides most other settings.";
        };

        allowedDomains = lib.mkOption {
          type = types.listOf types.str;
          default = [];
          description = "List of allowed domains";
        };

        autoUpdate = lib.mkOption {
          type = types.nullOr types.int;
          default = null;
          description = "Auto update interval in hours";
        };

        external = lib.mkOption {
          type = types.bool;
          default = true;
          description = "Allow external connections to the server";
        };

        filtersPath = lib.mkOption {
          type = types.nullOr types.str;
          default = null;
          description = "Path to the filters";
        };

        filterVersion = lib.mkOption {
          type = types.str;
          default = "OneHostsPro";
          description = "Filter version";
        };

        overrides = lib.mkOption {
          type = types.attrs;
          default = {};
          description = "List of domain overrides";
        };

        small = lib.mkOption {
          type = types.bool;
          default = true;
          description = "Small";
        };

        verbosity = lib.mkOption {
          type = types.int;
          default = 0;
          description = "Verbosity level";
        };

        webPassword = lib.mkOption {
          type = types.nullOr types.str;
          default = null;
          description = "Web password";
        };
      };
      config = lib.mkIf cfg.enable {
        systemd.services.macchiato-dns = {
          description = "A simple DNS proxy server written in rust. Macchiato DNS contains some powerful features that can be used to secure your communications.";
          wantedBy = [ "multi-user.target" ];

          serviceConfig = {
            ExecStart = "${cfg.package}/bin/dns --configuration ${configFile}";
            Restart = "on-failure";
            Type = "exec";
            User = "macchiato-dns";
            Group = "macchiato-dns";
            UMask = "0077";
            AmbientCapabilities = "CAP_NET_BIND_SERVICE";
            WorkingDirectory = toString cfg.package;

            # Security
            ProtectHome = "read-only";
            NoNewPrivileges = true;
            SystemCallArchitectures = "native";
            RestrictNamespaces = true;
            RestrictRealtime = true;
            RestrictSUIDSGID = true;
            ProtectControlGroups = true;
            ProtectHostname = true;
            ProtectKernelLogs = true;
            ProtectKernelModules = true;
            ProtectKernelTunables = true;
            LockPersonality = true;
            PrivateTmp = true;
          };
        };

        networking.firewall.allowedUDPPorts = mkIf cfg.openFirewall [ 53 ];
        networking.firewall.allowedTCPPorts = mkIf cfg.openWebFirewall [ 5554 ];

        users.users.macchiato-dns = {
          group = "macchiato-dns";
          isNormalUser = true;
        };
        users.groups.macchiato-dns = {};
      };
    };
  };
}
