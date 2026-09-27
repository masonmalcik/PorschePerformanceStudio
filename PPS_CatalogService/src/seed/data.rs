pub struct BrandSeed {
    pub code: &'static str,
    pub name: &'static str,
    pub image_name: &'static str,
    pub description: &'static str,
}
pub const BRANDS: &[BrandSeed] = &[BrandSeed {
    code: "PORSCHE",
    name: "Porsche",
    image_name: "pps-performance.png",
    description: "Porsche original-equipment and branded products",
}];

pub struct CategorySeed {
    pub code: &'static str,
    pub parent_code: Option<&'static str>,
    pub name: &'static str,
    pub description: &'static str,
}
pub const CATEGORIES: &[CategorySeed] = &[
    CategorySeed {
        code: "AIR_INTAKE",
        parent_code: None,
        name: "Air Intake",
        description: "Air intake system products",
    },
    CategorySeed {
        code: "BODY",
        parent_code: None,
        name: "Body",
        description: "Body and aerodynamic products",
    },
    CategorySeed {
        code: "BRAKES",
        parent_code: None,
        name: "Brakes",
        description: "Brake system products",
    },
    CategorySeed {
        code: "CHASSIS",
        parent_code: None,
        name: "Chassis",
        description: "Chassis reinforcement products",
    },
    CategorySeed {
        code: "COOLING",
        parent_code: None,
        name: "Cooling",
        description: "Cooling system products",
    },
    CategorySeed {
        code: "DRIVETRAIN",
        parent_code: None,
        name: "Drivetrain",
        description: "Drivetrain products",
    },
    CategorySeed {
        code: "ENGINE",
        parent_code: None,
        name: "Engine",
        description: "Engine products",
    },
    CategorySeed {
        code: "EXHAUST",
        parent_code: None,
        name: "Exhaust",
        description: "Exhaust system products",
    },
    CategorySeed {
        code: "FUEL",
        parent_code: None,
        name: "Fuel",
        description: "Fuel system products",
    },
    CategorySeed {
        code: "OIL",
        parent_code: None,
        name: "Oil",
        description: "Oil system products",
    },
    CategorySeed {
        code: "SUSPENSION",
        parent_code: None,
        name: "Suspension",
        description: "Suspension products",
    },
    CategorySeed {
        code: "TRANSMISSION",
        parent_code: None,
        name: "Transmission",
        description: "Transmission products",
    },
    CategorySeed {
        code: "VALVE_TRAIN",
        parent_code: Some("ENGINE"),
        name: "Valve Train",
        description: "Valve train products",
    },
    CategorySeed {
        code: "WHEELS_TIRES",
        parent_code: None,
        name: "Wheels and Tires",
        description: "Wheel and tire products",
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::path::Path;

    #[test]
    fn seed_codes_are_unique_and_parents_resolve() {
        let brand_codes: HashSet<_> = BRANDS.iter().map(|seed| seed.code).collect();
        assert_eq!(brand_codes.len(), BRANDS.len());
        let category_codes: HashSet<_> = CATEGORIES.iter().map(|seed| seed.code).collect();
        assert_eq!(category_codes.len(), CATEGORIES.len());
        for seed in CATEGORIES {
            if let Some(parent) = seed.parent_code {
                assert!(category_codes.contains(parent), "missing parent {parent}");
            }
        }
        for seed in BRANDS {
            let path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("assets")
                .join("brands")
                .join(seed.image_name);
            assert!(path.is_file(), "missing brand image {}", path.display());
        }
    }
}
