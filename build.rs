extern crate cc;
extern crate bindgen;

use std::env;
use std::path::PathBuf;
// use std::process::Command;

fn main() {
    // Command::new("sh").args(&["pre_build.sh"])
    //                     .status().unwrap();
    #[cfg(feature = "generate-bindings")]
    generate_bindings();
    build();
}

#[cfg(feature = "generate-bindings")]
fn generate_bindings(){
    let bindings = bindgen::Builder::default()
        .header("cglucose/wrapper.h")
        .allowlist_type("*CGlucose*")
        .allowlist_function("cglucose_init")
        .allowlist_function("cglucose_assume")
        .allowlist_function("cglucose_solve")
        .allowlist_function("cglucose_val")
        .allowlist_function("cglucose_add_to_clause")
        .allowlist_function("cglucose_commit_clause")
        .allowlist_function("cglucose_clean_clause")
        .allowlist_function("cglucose_solver_nodes")
        .allowlist_function("cglucose_nb_learnt")
        .allowlist_function("cglucose_set_random_seed")
        .allowlist_function("cglucose_clean_clause_receive")
        .allowlist_function("cglucose_clean_clause_send")
        .allowlist_function("cglucose_add_to_clause_receive")
        .allowlist_function("cglucose_add_to_clause_send")
        .allowlist_function("cglucose_print_incremental_stats")
        .allowlist_function("cglucose_commit_incoming_clause")
        //.allowlist_function("cglucose_get_add_conflicts_size")
        //.allowlist_function("cglucose_get_conflicts_at")
        .rustfmt_bindings(true)
        // .enable_cxx_namespaces()
        .clang_arg("-Icglucose/")
        // .clang_arg(r"-std=c++11")
        // .clang_arg("-xc++")
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("glucose_bindings.rs"))
        .expect("Couldn't write bindings!");
}

fn build(){
    cc::Build::new()
        .include("cglucose/")
        .cpp(true)
        .file("cglucose/simp/SimpSolver.cc")
        .file("cglucose/simp/SolverHelper.cc")
        .file("cglucose/utils/System.cc")
        .file("cglucose/utils/Options.cc")
        .file("cglucose/core/Solver.cc")
        .file("cglucose/wrapper.cpp")
        .flag_if_supported("-D__STDC_LIMIT_MACROS")
        .flag_if_supported("-D__STDC_FORMAT_MACROS")
        .flag_if_supported("-DNDEBUG")
        .flag_if_supported("-fomit-frame-pointer")
        .flag_if_supported("-std=c++11")
        .flag_if_supported("-w")
        .opt_level(3)
        .compile("glucose");
}