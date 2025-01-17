// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use indoc::indoc;
use ispm_wrapper::data::ProjectPackage;
use simics_test::TestEnvSpec;
use std::path::PathBuf;

#[test]
#[cfg_attr(miri, ignore)]
fn test_riscv_64_userspace_magic() -> Result<()> {
    let output = TestEnvSpec::builder()
        .name("test_riscv_64_userspace_magic")
        .package_crates([PathBuf::from(env!("CARGO_MANIFEST_DIR"))])
        .packages([
            ProjectPackage::builder()
                .package_number(1000)
                .version("latest")
                .build(),
            ProjectPackage::builder()
                .package_number(2050)
                .version("latest")
                .build(),
            ProjectPackage::builder()
                .package_number(2053)
                .version("latest")
                .build(),
        ])
        .cargo_target_tmpdir(env!("CARGO_TARGET_TMPDIR"))
        .directories([PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("rsrc")
            .join("riscv-64")])
        .build()
        .to_env()?
        .test(indoc! {r#"
            load-module tsffs
            init-tsffs

            @tsffs.log_level = 2
            @tsffs.start_on_harness = True
            @tsffs.stop_on_harness = True
            @tsffs.timeout = 3.0
            @tsffs.exceptions = [14]
            @tsffs.generate_random_corpus = True
            @tsffs.iteration_limit = 1000
            @tsffs.use_snapshots = True

            load-target "risc-v-simple/linux" namespace = riscv machine:hardware:storage:disk1:image = "test.fs.craff"

            script-branch {
                bp.time.wait-for seconds = 15
                board.console.con.input "mkdir /mnt/disk0\r\n"
                bp.time.wait-for seconds = 1.0
                board.console.con.input "mount /dev/vdb /mnt/disk0\r\n"
                bp.time.wait-for seconds = 1.0
                board.console.con.capture-start out.txt
                board.console.con.input "/mnt/disk0/test\r\n"
            }

            script-branch {
                bp.time.wait-for seconds = 240
                quit 1
            }

            run
        "#})?;

    let output_str = String::from_utf8_lossy(&output.stdout);

    println!("{output_str}");

    Ok(())
}
