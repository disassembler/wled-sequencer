{
  flake = { config, ... }: {
    nixosModules = let
      name = "wled-sequencer";
    in {
      default = {
        imports = [
          config.nixosModules.${name}
        ];
      };

      ${name} = { config, lib, pkgs, ... }: let
        cfg = config.services.${name};
      in {
        options.services.${name} = {
            enable = lib.mkEnableOption name;
        
            package = lib.mkPackageOption pkgs name { };
        
            settings = {
              host = lib.mkOption {
                type = lib.types.str;
                description = "IP address of the WLED controller (corresponds to --host).";
                example = "127.0.0.1";
              };
        
              port = lib.mkOption {
                type = lib.types.ints.u16;
                description = "UDP port for the Distributed Display Protocol (DDP) (corresponds to --port).";
                default = 4048;
              };
        
              file = lib.mkOption {
                type = lib.types.path;
                description = "Path to the FSEQ sequence file. (Corresponds to --file).";
                example = "/home/user/sequences/my_show.fseq";
              };
        
              loop-enabled = lib.mkOption {
                type = lib.types.bool;
                description = "Enable continuous looping of the FSEQ sequence (corresponds to --loop-enabled).";
                default = true;
              };
            };
          };
          config = lib.mkIf cfg.enable {
            systemd.services.${name} = lib.mkIf cfg.enable {
              after = [ "network.target" ];
              wantedBy = [ "multi-user.target" ];
        
              path = [ cfg.package pkgs.iputils ];
        
              # Build the execution command based on settings
              script = toString [
                "exec"
                (with lib; pipe cfg.package [getExe builtins.baseNameOf escapeShellArg])
		(lib.cli.toGNUCommandLineShell {} cfg.settings)
              ];
                
                serviceConfig = {
                  Type = "exec";
                  StateDirectory = name;
                  DynamicUser = true;
                  Restart = "always";
                };
              };
          };
        };
    };
  };
}
