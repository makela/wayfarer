# NixOS module running copies of the blog as hardened systemd services, one
# per instance. Import it in the server's configuration.nix, e.g. from a clone
# of this repository, and put a reverse proxy such as nginx in front:
#
#   imports = [ /srv/wayfarer/nix/module.nix ];
#   services.taichiwayfarer.instances = {
#     www = {
#       siteUrl = "https://www.taichiwayfarer.com";
#       # Gmail with an app password (needs 2-Step Verification on the account)
#       smtp = {
#         host = "smtp.gmail.com";
#         user = "taichiwayfarer@gmail.com";
#         passwordFile = "/var/lib/secrets/taichiwayfarer/gmail-app-password";
#       };
#     };
#     alt = {
#       package = pkgs.callPackage /srv/wayfarer-alt/nix/package.nix { };
#       port = 3001;
#       siteUrl = "https://alt.taichiwayfarer.com";
#       noindex = true;
#     };
#   };
#
# Each instance runs as the service taichiwayfarer-<name>. Its database,
# e.g. of subscribers to new posts, is
# /var/lib/taichiwayfarer-<name>/taichiwayfarer.db, and
# `sudo taichiwayfarer-<name>-subscribers list | import | remove <email>`
# manages them (see `subscribers_command` in src/main.rs), e.g.
#
#   sudo taichiwayfarer-www-subscribers import < emails.txt
#
# Without smtp.host, as alt above, the instance only logs its emails.
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

        mail = {
          from = lib.mkOption {
            type = lib.types.str;
            default = "Tai Chi Wayfarer <taichiwayfarer@gmail.com>";
            description = ''
              Who the emails to subscribers are from (MAIL_FROM); replies go there too.
              With Gmail, the account's own address.
            '';
          };
          notify = lib.mkOption {
            type = lib.types.nullOr lib.types.str;
            default = null;
            example = "timo@example.com";
            description = "Who hears of new subscribers and their notes (NOTIFY_EMAIL), or null for mail.from.";
          };
        };

        smtp = {
          host = lib.mkOption {
            type = lib.types.nullOr lib.types.str;
            default = null;
            example = "smtp.gmail.com";
            description = "SMTP server the emails are sent through, or null to only log them.";
          };
          port = lib.mkOption {
            type = lib.types.nullOr lib.types.port;
            default = null;
            description = "Port of the SMTP server, or null for 465 with tls and 587 otherwise.";
          };
          encryption = lib.mkOption {
            type = lib.types.enum [
              "tls"
              "starttls"
              "none"
            ];
            default = "tls";
            description = "tls (usually port 465, as Gmail), starttls (usually 587), or none for a local relay.";
          };
          user = lib.mkOption {
            type = lib.types.nullOr lib.types.str;
            default = null;
            description = "SMTP login, or null for a server that needs none.";
          };
          passwordFile = lib.mkOption {
            type = lib.types.nullOr lib.types.str;
            default = null;
            example = "/var/lib/secrets/taichiwayfarer/gmail-app-password";
            description = ''
              File with the SMTP password, e.g. a Gmail app password, outside the Nix
              store. The service reads it through systemd credentials. Needed with a user.
            '';
          };
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
      smtpPassword = instance.smtp.host != null && instance.smtp.passwordFile != null;
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
        DATABASE_URL = databaseUrl name;
        MAIL_FROM = instance.mail.from;
      }
      // lib.optionalAttrs (instance.mail.notify != null) {
        NOTIFY_EMAIL = instance.mail.notify;
      }
      // lib.optionalAttrs (instance.smtp.host != null) {
        SMTP_HOST = instance.smtp.host;
        SMTP_ENCRYPTION = instance.smtp.encryption;
      }
      // lib.optionalAttrs (instance.smtp.host != null && instance.smtp.port != null) {
        SMTP_PORT = toString instance.smtp.port;
      }
      // lib.optionalAttrs (instance.smtp.host != null && instance.smtp.user != null) {
        SMTP_USER = instance.smtp.user;
      }
      // lib.optionalAttrs smtpPassword {
        # %d is the service's credentials directory, see LoadCredential.
        SMTP_PASSWORD_FILE = "%d/smtp-password";
      };

      serviceConfig = {
        ExecStart = lib.getExe instance.package;
        Restart = "on-failure";

        # The server reads from the Nix store, keeps its subscribers in its
        # state directory, listens on a port and sends emails.
        DynamicUser = true;
        # Named, so that the subscribers command runs as the same user.
        User = "taichiwayfarer-${name}";
        StateDirectory = "taichiwayfarer-${name}";
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
          # Looking up the SMTP server's address, e.g. through nscd.
          "AF_UNIX"
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
      }
      // lib.optionalAttrs smtpPassword {
        LoadCredential = "smtp-password:${instance.smtp.passwordFile}";
      };
    };

  databaseUrl = name: "sqlite:///var/lib/taichiwayfarer-${name}/taichiwayfarer.db";

  # taichiwayfarer-<name>-subscribers: the subscribers command, run as the
  # service's user so that the database stays the service's. Run with sudo.
  subscribersCommand =
    name: instance:
    pkgs.writeShellScriptBin "taichiwayfarer-${name}-subscribers" ''
      exec ${config.systemd.package}/bin/systemd-run --quiet --pipe --wait --collect \
        --property=DynamicUser=yes \
        --property=User=taichiwayfarer-${name} \
        --property=StateDirectory=taichiwayfarer-${name} \
        --property=UMask=0077 \
        --setenv=DATABASE_URL=${databaseUrl name} \
        --setenv=RUST_LOG=warn \
        ${lib.getExe instance.package} subscribers "$@"
    '';

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
    ]
    ++ lib.mapAttrsToList (name: instance: {
      assertion = instance.smtp.user == null || instance.smtp.passwordFile != null;
      message = "services.taichiwayfarer.instances.${name}.smtp: a user needs a passwordFile.";
    }) instances;

    systemd.services = lib.mapAttrs' service instances;
    environment.systemPackages = lib.mapAttrsToList subscribersCommand instances;
  };
}
