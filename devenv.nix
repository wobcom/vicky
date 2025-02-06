{ config, pkgs, ... }:

{
  services.minio.enable = true;
  services.minio.buckets = [ "vicky-logs" ];
  services.minio.accessKey = "minio";
  services.minio.secretKey = "aichudiKohr6aithi4ahh3aeng2eL7xo";

  services.postgres.enable = true;
  services.postgres.initialDatabases = [
    {
      name = "vicky";
      user = "vicky";
      pass = "vicky";
    }
  ];

}
