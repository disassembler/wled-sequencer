{ inputs, ... }: {
  perSystem = { system, config, lib, pkgs, ... }: {
    packages = {
      libtinyfseq = pkgs.stdenv.mkDerivation rec {
        pname = "libtinyfseq";
        version = "1.0.0"; 
  
        # Fetch the source from GitHub. 
        # NOTE: You MUST replace the 'rev' (commit hash) and 'hash' with actual values 
        # for the version of libtinyfseq you want to use.
        src = pkgs.fetchFromGitHub {
          owner = "Cryptkeeper";
          repo = "libtinyfseq";
          rev = "9e1f0e25c6f589bc639412beaac8c43ed2c854f4"; # Placeholder, replace with actual commit
          hash = "sha256-p4RQrhsvl5Ieq1PbrJFcP9Rll3ohZXCom52+AjerphQ="; # Placeholder, use nix-prefetch-url or similar
        };

	buildPhase = ''
          # 1. Create a C file that defines the implementation macro and includes the header.
          # This is the single source file that contains all compiled code.
          echo '#define TINYFSEQ_IMPLEMENTATION' > tinyfseq_impl.c
          cat tinyfseq.h >> tinyfseq_impl.c

          # 2. Compile the implementation file into a static object (.o).
          # We use the targetPrefix for cross-compilation support (standard Nix practice).
          ${pkgs.stdenv.cc.targetPrefix}gcc -c tinyfseq_impl.c -o tinyfseq.o -std=c99
          
          # 3. Archive the object file into a static library (.a).
          ${pkgs.stdenv.cc.targetPrefix}ar rcs libtinyfseq.a tinyfseq.o
        '';

        # 🛠️ FIX: Install the files built in the buildPhase.
        installPhase = ''
          # 1. Install the manually built static library
          mkdir -p $out/lib
          cp libtinyfseq.a $out/lib/
          
          # 2. Install the header file for bindgen (which needs to be named as expected).
          mkdir -p $out/include/libtinyfseq
          cp tinyfseq.h $out/include/libtinyfseq/tinyfseq.h
        '';
      };
      wled-sequencer = let
        naersk-lib = inputs.naersk.lib.${system};
      in naersk-lib.buildPackage rec {
        pname = "wled-sequencer";

        src = with lib.fileset; toSource {
          root = ./..;
          fileset = unions [
            ../Cargo.lock
            ../Cargo.toml
            ../src
            ../build.rs
          ];
        };

	TINYFSEQ_INCLUDE_DIR="${config.packages.libtinyfseq}/include/libtinyfseq";
	TINYFSEQ_LIB_DIR="${config.packages.libtinyfseq}/lib";
      	LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
      	BINDGEN_EXTRA_CLANG_ARGS = "-I${pkgs.stdenv.cc.libc.dev}/include";
        buildInputs = with pkgs; [
          pkg-config
          zlib
	  config.packages.libtinyfseq
        ];

        meta = {
          mainProgram = pname;
          maintainers = with lib.maintainers; [
            disassembler
          ];
          license = with lib.licenses; [
            asl20
          ];
        };
      };
    };
  };
}
