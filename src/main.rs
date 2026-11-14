//! Binary entry point for r0-wasm. Runs the pedon demo.

use r0_wasm::{R0Vm, DEFAULT_FUEL};

fn main() {
    let mut vm = R0Vm::new(DEFAULT_FUEL);
    let r0_script = r#"
        depth_weighted_mean <- function(hz: df, z_min: dbl = 0, z_max: dbl = 100) -> dbl {
            o_top <- pmax(hz$top, z_min)
            o_bot <- pmin(hz$bot, z_max)
            thick <- pmax(o_bot - o_top, 0)
            valid <- !is.na(thick) & !is.na(hz$clay) & (thick > 0)
            valid_thick <- thick[valid]
            total_thick <- valid_thick |> sum(na.rm = TRUE)
            if (total_thick <= 0) {
                return(NA_real_)
            }
            sum(valid_thick * hz$clay[valid]) / total_thick
        }
        pedon <- data.frame(
            top = c(0, 18, 42, 75, 110),
            bot = c(18, 42, 75, 110, 150),
            clay = c(14.5, 24.0, 31.2, 28.0, NA_real_)
        )
        wt_clay_100 <- pedon |> depth_weighted_mean(0, 100)
        midpoints   <- (pedon$top + pedon$bot) / 2
        valid_idx   <- which(!is.na(pedon$clay))
        spline_pts <- approx(midpoints[valid_idx], pedon$clay[valid_idx], c(25, 50, 85))
        list(
            wt_clay_0_100cm = wt_clay_100,
            interp_25_50_85 = spline_pts,
            remaining_oob   = pedon$clay[c(1L, 5L, 99L)]
        )
    "#;

    let mut failed = false;
    match vm.eval_script(r0_script) {
        Ok(res) => {
            println!("=== R0 Execution Succeeded (Fuel remaining: {}) ===", vm.fuel);
            println!("{}", res);
        }
        Err(e) => {
            eprintln!("R0 Execution Trap: {}", e);
            failed = true;
        }
    }

    // Strict broadcasting must trap; this is the expected demonstration.
    let bad_recycle = "c(1, 2, 3, 4) + c(10, 20)";
    match vm.eval_script(bad_recycle) {
        Ok(v) => println!("Strict Broadcast Check (`{}`): unexpected value {}", bad_recycle, v),
        Err(e) => println!("Strict Broadcast Check (`{}`): {}", bad_recycle, e),
    }

    if failed {
        std::process::exit(1);
    }
}
