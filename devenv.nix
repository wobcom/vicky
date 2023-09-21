{ pkgs, ... }:

{
  services.minio.enable = true;
  services.minio.buckets = [ "testbucket" ];

  processes = {
    etcd.exec = "${pkgs.etcd}/bin/etcd --listen-client-urls=http://127.0.0.1:2379 --advertise-client-urls=http://127.0.0.1:2379";
  };
}
