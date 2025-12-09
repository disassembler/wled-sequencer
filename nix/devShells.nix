{
  perSystem = { config, pkgs, ... }: {
    devShells.default = with pkgs; mkShell {
      packages = [
        cargo
        cmake
        rustc
        pkg-config
        openssl
        zlib
        rust-analyzer
        rustfmt
	libclang
        clippy
	config.packages.libtinyfseq
	clang-tools
      ];
      LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
      BINDGEN_EXTRA_CLANG_ARGS = "-I${pkgs.stdenv.cc.libc.dev}/include";
      TINYFSEQ_INCLUDE_DIR = "${config.packages.libtinyfseq}/include/libtinyfseq";
      TINYFSEQ_LIB_DIR = "${config.packages.libtinyfseq}/lib";
    };
  };
}
