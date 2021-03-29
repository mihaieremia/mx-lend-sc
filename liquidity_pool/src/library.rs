elrond_wasm::imports!();

// base precision 
const BP: u32 = 100000;

#[elrond_wasm_derive::module(LibraryModuleImpl)]
pub trait LibraryModule {
    fn init(&self) {}

    fn compute_borrow_rate(
        &self,
        r_base: BigUint,
        r_slope1: BigUint,
        r_slope2: BigUint,
        u_optimal: BigUint,
        u_current: BigUint,
    ) -> BigUint {
        let bp = BigUint::from(BP);

        let borrow_rate: BigUint;
        if u_current < u_optimal {
            let utilisation_ratio = (u_current.clone() * r_slope1) / u_optimal.clone();
            borrow_rate = r_base + utilisation_ratio;
        } else {
            let denominator = bp - u_optimal.clone();
            let numerator = (u_current.clone() - u_optimal.clone()) * r_slope2;
            borrow_rate = (r_base + r_slope1) + numerator / denominator;
        }

        return borrow_rate;
    }

    fn compute_deposit_rate(
        &self,
        u_current: BigUint,
        borrow_rate: BigUint,
        reserve_factor: BigUint,
    ) -> BigUint {
        let bp = BigUint::from(BP);
        let loan_ratio = u_current.clone() * borrow_rate;
        let deposit_rate = u_current.clone() * loan_ratio * (bp.clone() - reserve_factor);
        return deposit_rate / (bp.clone() * bp.clone() * bp.clone());
    }

    fn compute_capital_utilisation(
        &self,
        borrowed_amount: BigUint,
        total_pool_reserves: BigUint,
    ) -> BigUint {
        let bp = BigUint::from(BP);
        return BigUint::from((borrowed_amount * bp) / total_pool_reserves);
    }
}
