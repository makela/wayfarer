# NixOS module running copies of the blog as hardened systemd services, one
# per instance. Import it in the server's configuration.nix, e.g. from a clone
# of this repository, and put a reverse proxy such as nginx in front:
#
#   imports = [ /srv/wayfarer/nix/module.nix ];
#   services.taichiwayfarer.instances = {
#     www.siteUrl = "https://www.taichiwayfarer.com";
#     alt = {
#       package = pkgs.callPackage /srv/wayfarer-alt/nix/package.nix { };
#       port = 3001;
#       siteUrl = "https://alt.taichiwayfarer.com";
#       noindex = true;
#     };
#   };
#
# Each instance runs as the service taichiwayfarer-<name>.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  instances = lib.filterAttrs (_: instance: instance.enable) config.services.taichiwayfarer.instances;

  instanceOptions =
    { name, ... }:
    {
      options = {
        enable = lib.mkOption {
          type = lib.types.bool;
          default = true;
          description = "Whether to run this instance.";
        };

        package = lib.mkOption {
          type = lib.types.package;
          default = pkgs.callPackage ./package.nix { };
          defaultText = lib.literalExpression "pkgs.callPackage ./package.nix { }";
          description = "The blog package, including its posts and static files.";
        };

        siteUrl = lib.mkOption {
          type = lib.types.str;
          example = "https://www.taichiwayfarer.com";
          description = "Public address of the site, used for absolute links, the feed and the sitemap.";
        };

        noindex = lib.mkOption {
          type = lib.types.bool;
          default = false;
          description = "Ask search engines not to index this instance, e.g. for a test copy.";
        };

        host = lib.mkOption {
          type = lib.types.str;
          default = "127.0.0.1";
          description = "Address to listen on. The default only accepts local connections, e.g. from a reverse proxy.";
        };

        port = lib.mkOption {
          type = lib.types.port;
          default = 3000;
          description = "Port to listen on (instance ${name}).";
        };

        logLevel = lib.mkOption {
          type = lib.types.str;
          default = "taichiwayfarer=info,tower_http=info";
          description = "Log filter (RUST_LOG). Logs go to the journal.";
        };
      };
    };

  service =
    name: instance:
    let
      share = "${instance.package}/share/taichiwayfarer";
    in
    lib.nameValuePair "taichiwayfarer-${name}" {
      description = "Tai Chi Wayfarer blog (${name})";
      wantedBy = [ "multi-user.target" ];
      after = [ "network.target" ];

      environment = {
        HOST = instance.host;
        PORT = toString instance.port;
        SITE_URL = instance.siteUrl;
        NOINDEX = if instance.noindex then "1" else "0";
        POST_DIR = "${share}/posts";
        STATIC_DIR = "${share}/dist";
        RUST_LOG = instance.logLevel;
      };

      serviceConfig = {
        ExecStart = lib.getExe instance.package;
        Restart = "on-failure";

        # The server only reads from the Nix store and listens on a port.
        DynamicUser = true;
        WorkingDirectory = "/";
        CapabilityBoundingSet = "";
        NoNewPrivileges = true;
        PrivateDevices = true;
        PrivateTmp = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        ProtectClock = true;
        ProtectHostname = true;
        ProtectKernelLogs = true;
        ProtectKernelModules = true;
        ProtectKernelTunables = true;
        ProtectControlGroups = true;
        ProtectProc = "invisible";
        RestrictAddressFamilies = [
          "AF_INET"
          "AF_INET6"
        ];
        RestrictNamespaces = true;
        RestrictRealtime = true;
        RestrictSUIDSGID = true;
        LockPersonality = true;
        MemoryDenyWriteExecute = true;
        SystemCallArchitectures = "native";
        SystemCallFilter = [
          "@system-service"
          "~@privileged"
        ];
        UMask = "0077";
      };
    };

  # host:port of each instance, to catch two instances on the same port.
  addresses = lib.mapAttrsToList (_: instance: "${instance.host}:${toString instance.port}") instances;
in
{
  options.services.taichiwayfarer.instances = lib.mkOption {
    type = lib.types.attrsOf (lib.types.submodule instanceOptions);
    default = { };
    description = "Copies of the blog to run, by name.";
  };

  config = lib.mkIf (instances != { }) {
    assertions = [
      {
        assertion = lib.length addresses == lib.length (lib.unique addresses);
        message = "services.taichiwayfarer.instances: each instance needs its own port (${lib.concatStringsSep ", " addresses}).";
      }
    ];

    systemd.services = lib.mapAttrs' service instances;
  };
}
