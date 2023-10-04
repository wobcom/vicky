{ config, pkgs, ... }:

{
  services.minio.enable = true;
  services.minio.buckets = [ "vicky-logs" ];

  env.ETCD_DATA_DIR = config.env.DEVENV_STATE + "/etcd";

  processes = {
    etcd.exec = "${pkgs.etcd}/bin/etcd --listen-client-urls=http://127.0.0.1:2379 --advertise-client-urls=http://127.0.0.1:2379";
  };
}
