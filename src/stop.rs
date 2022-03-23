use std::ops::{DerefMut, Deref};

use super::{
    *,
    energy::*
};

#[repr(C)]
pub struct FighterKineticEnergyStop {
    parent: super::energy::KineticEnergy,
    padding: u64,
    _x90: PaddedVec2,
    reset_type: EnergyStopResetType,
    elapsed_hitstop_frames: f32,
    hitstop_frames: f32,
    _xAC: f32,
    _xB0: f32,
    _xB4: u32,
    _xB8: u16,
    _xBA: bool,
    _xBB: bool,
    _xBC: u32,
    _xC0: PaddedVec2
    // ...
}

impl Deref for FighterKineticEnergyStop {
    type Target = super::energy::KineticEnergy;

    fn deref(&self) -> &Self::Target {
        &self.parent
    }
}

impl DerefMut for FighterKineticEnergyStop {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.parent
    }
}

impl FighterKineticEnergyStop {
    pub fn get_parent_sum_speed_correct(boma: &mut BattleObjectModuleAccessor, link_no: i32, arg: i32) -> PaddedVec2 {
        unsafe {
            let func: extern "C" fn(&mut BattleObjectModuleAccessor, i32, i32) -> energy::Vec3 = std::mem::transmute(LinkModule::get_parent_sum_speed as *const ());
            let vec = func(boma, link_no, arg);
            PaddedVec2::new(vec.x, vec.y)
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum EnergyStopResetType {
    Ground = 0x0,
    DamageGround,
    DamageAir,
    DamageAirIce,
    DamageOther,
    DamageKnockBack,
    GlidLanding,
    Air,
    AirXNormalMax,
    AirEscape,
    AirBrake,
    AirBrakeAlways,
    GuardDamage,
    Capture,
    CatchCut,
    ItemSwingDash,
    ItemDashThrow,
    SwimBrake,
    Run,
    RunBrake,
    GlidStart,
    CatchDash,
    ShieldRebound,
    Free,
    CaptureBeetle,
    AirLassoHang,
    AirLassoRewind,
    EscapeAirSlide,
    DamageGroundOrbit,
    DamageAirOrbit,
}

#[skyline::from_offset(0x3ac540)]
unsafe extern "C" fn get_battle_object_from_id(id: u32) -> *mut BattleObject;

#[cfg(feature = "dev-plugin")]
#[no_mangle]
pub unsafe extern "Rust" fn update_stop(energy: &mut FighterKineticEnergyStop, boma: &mut BattleObjectModuleAccessor) -> bool {
    use EnergyStopResetType::*;

    match energy.reset_type {
        DamageKnockBack => loop {
            if 0.0 >= energy.hitstop_frames {
                break;
            }

            if StatusModule::situation_kind(boma) != *SITUATION_KIND_GROUND || energy.hitstop_frames <= energy.elapsed_hitstop_frames {
                energy.speed.x = 0.0;
                energy.elapsed_hitstop_frames = 0.0;
                energy.hitstop_frames = 0.0;
                energy._xAC = 0.0;
                energy._xB0 = 0.0
            } else {
                let rate = WorkModule::get_param_float(boma, smash::hash40("common"), smash::hash40("damage_knock_back_speed_x_rate"));
                let progress = energy.elapsed_hitstop_frames / energy.hitstop_frames;
                let remaining_progress = 1.0 - progress;
                // A(1-x)^3 + Bx(1-x)^2 + C(1-x)x^2 + Dx^3 
                // A = damage_knock_back_speed_x_rate * 0.01
                // B = 3.0 * 0.6 = 1.8
                // C = 3.0 * 0.79 = ~2.4
                // D = 1.0
                let speed_x = rate * 0.01 * remaining_progress.powi(3);
                let speed_x = speed_x + 3.0 * 0.6 * progress * remaining_progress.powi(2);
                let speed_x = speed_x + 3.0 * 0.79 * progress.powi(2) * remaining_progress;
                let speed_x = speed_x + progress.powi(3);
                let speed_x = speed_x * energy._xB0;

                // not sure about this one chief
                energy.rot_speed = energy.speed;

                energy.speed = PaddedVec2::new(
                    speed_x - energy._xAC,
                    energy.speed.y
                );


                energy.elapsed_hitstop_frames += 1.0;
                energy._xAC = speed_x;
            }
            break;
        },
        AirXNormalMax => {
            let speed = energy.get_speed();
            let brake = if speed.x.abs() <= WorkModule::get_param_float(boma, smash::hash40("air_speed_x_stable"), 0) {
                WorkModule::get_param_float(boma, smash::hash40("air_brake_x"), 0)
            } else {
                WorkModule::get_param_float(boma, smash::hash40("common"), smash::hash40("fall_brake_x"))
            };
            energy.speed_brake = PaddedVec2::new(brake, 0.0);
        },
        ItemSwingDash | ItemDashThrow => {
            if crate::motion::FighterKineticEnergyMotion::is_main_motion_updating_energy(boma) {
                MotionModule::update_trans_move_speed(boma);
                let speed = crate::motion::FighterKineticEnergyMotion::trans_move_speed_correct(boma);
                let lr = PostureModule::lr(boma);
                let energy_speed = energy.get_speed();
                let accel = PaddedVec2::new(speed.z - energy_speed.x, speed.y - energy_speed.y);
                energy.speed_max = PaddedVec2::new(-energy_speed.x, -energy_speed.y);
                energy.speed_brake = PaddedVec2::zeros();
                energy.accel = accel;
            }
            if energy.reset_type == ItemDashThrow
            && MotionModule::frame(boma) > WorkModule::get_param_int(boma, smash::hash40("common"), smash::hash40("item_dash_throw_brake_dec_frame")) as f32
            {
                let brake = WorkModule::get_param_float(boma, smash::hash40("ground_brake"), 0)
                                    * WorkModule::get_param_float(boma, smash::hash40("common"), smash::hash40("item_dash_throw_brake_mul"))
                                    * WorkModule::get_param_float(boma, smash::hash40("common"), smash::hash40("item_dash_throw_brake_dec"));
                energy.speed_brake = PaddedVec2::new(brake, 0.0);
            }
        },
        CaptureBeetle => {
            if LinkModule::is_link(boma, *LINK_NO_CAPTURE) {
                energy.speed = FighterKineticEnergyStop::get_parent_sum_speed_correct(boma, *LINK_NO_CAPTURE, 1);
                return true;
            }
        }
        _ => return false
    }

    energy.process(boma);

    let status_module = *(boma as *const BattleObjectModuleAccessor as *const u64).add(0x8);
    if !*(status_module as *const bool).add(0x12a) {
        if StatusModule::situation_kind(boma) == *SITUATION_KIND_AIR {
            let horizontal_limit = WorkModule::get_param_float(boma, smash::hash40("common"), smash::hash40("common_air_speed_x_limit"));
            let vertical_limit = if energy.speed.y <= 0.0 {
                WorkModule::get_param_float(boma, smash::hash40("common"), smash::hash40("air_speed_down_limit"))
            } else {
                WorkModule::get_param_float(boma, smash::hash40("common"), smash::hash40("air_speed_up_limit"))
            };

            if horizontal_limit < energy.speed.x.abs() {
                energy.speed.x = vertical_limit * energy.speed.x.signum();
            }

            if vertical_limit < energy.speed.y.abs() {
                energy.speed.y = vertical_limit * energy.speed.y.signum();
            }
        } else if StatusModule::situation_kind(boma) == *SITUATION_KIND_GROUND {
            let speed_limit = WorkModule::get_param_float(boma, smash::hash40("common"), smash::hash40("ground_speed_limit"));
            if speed_limit < energy.speed.x.abs() {
                energy.speed.x = speed_limit * energy.speed.x.signum();
            }
        }
    }

    true

    // println!("{:?}", energy.reset_type);


    // false
}

#[cfg(feature = "dev-plugin")]
#[no_mangle]
pub unsafe extern "Rust" fn initialize_stop(energy: &mut FighterKineticEnergyStop, boma: &mut BattleObjectModuleAccessor) -> bool {
    use EnergyStopResetType::*;
    
    match energy.reset_type {
        Ground | CatchCut | ItemSwingDash | ItemDashThrow => {
            let ground_brake = WorkModule::get_param_float(boma, smash::hash40("ground_brake"), 0);
            let mut multiplier = match energy.reset_type {
                CatchCut => WorkModule::get_param_float(boma, smash::hash40("common"), smash::hash40("capture_cut_brake_mul")),
                ItemSwingDash => WorkModule::get_param_float(boma, smash::hash40("common"), smash::hash40("item_dash_swing_brake_mul")),
                ItemDashThrow => WorkModule::get_param_float(boma, smash::hash40("common"), smash::hash40("item_dash_throw_brake_mul")),
                _ => 1.0
            };
            if energy._xBB {
                multiplier *= WorkModule::get_param_float(boma, smash::hash40("common"), smash::hash40("stop_over_speed_brake_mul"));
            }
            energy.speed_brake = PaddedVec2::new(ground_brake * multiplier, 0.0);
            energy.speed_limit = PaddedVec2::new(
                WorkModule::get_param_float(boma, smash::hash40("common"), smash::hash40("ground_speed_limit")),
                0.0
            );
        },
        DamageGround | GuardDamage | DamageGroundOrbit => {
            let brake = WorkModule::get_param_float(boma, smash::hash40("ground_brake"), 0)
                                * WorkModule::get_param_float(boma, smash::hash40("common"), smash::hash40("damage_ground_mul"));
            energy.speed_brake = PaddedVec2::new(brake, 0.0);
            energy.speed_limit = PaddedVec2::new(
                WorkModule::get_param_float(boma, smash::hash40("battle_object"), smash::hash40("damage_speed_limit")),
                0.0
            );
        },
        Air | AirXNormalMax => {
            energy.speed_limit = PaddedVec2::new(
                WorkModule::get_param_float(boma, smash::hash40("common"), smash::hash40("air_speed_x_limit")),
                0.0
            );
            energy.speed_brake = PaddedVec2::new(
                WorkModule::get_param_float(boma, smash::hash40("air_brake_x"), 0),
                0.0
            );
        },
        AirEscape => {
            energy.speed_limit = PaddedVec2::new(
                WorkModule::get_param_float(boma, smash::hash40("common"), smash::hash40("air_speed_x_limit")),
                0.0
            );
            energy.speed_brake = PaddedVec2::new(
                WorkModule::get_param_float(boma, smash::hash40("common"), smash::hash40("escape_air_brake")),
                0.0
            );
        },
        Run => {
            energy.speed_limit = PaddedVec2::new(
                WorkModule::get_param_float(boma, smash::hash40("common"), smash::hash40("ground_speed_limit")),
                0.0
            );
        },
        RunBrake => {
            let brake = WorkModule::get_param_float(boma, smash::hash40("ground_brake"), 0)
                                * WorkModule::get_param_float(boma, smash::hash40("common"), smash::hash40("run_brake_brake_mul"));
            energy.speed_brake = PaddedVec2::new(
                brake,
                0.0
            );
            energy.speed_limit = PaddedVec2::new(
                WorkModule::get_param_float(boma, smash::hash40("common"), smash::hash40("ground_speed_limit")),
                0.0
            );
        },
        CatchDash => {
            let brake = WorkModule::get_param_float(boma, smash::hash40("ground_brake"), 0)
                                * WorkModule::get_param_float(boma, smash::hash40("common"), smash::hash40("catch_dash_brake_mul"));
            energy.speed_brake = PaddedVec2::new(
                brake,
                0.0
            );
            energy.speed_limit = PaddedVec2::new(
                WorkModule::get_param_float(boma, smash::hash40("common"), smash::hash40("ground_speed_limit")),
                0.0
            );
        },
        ShieldRebound => {
            let brake = WorkModule::get_param_float(boma, smash::hash40("ground_brake"), 0)
                                * WorkModule::get_param_float(boma, smash::hash40("common"), smash::hash40("shield_rebound_ground_brake"));
            energy.speed_brake = PaddedVec2::new(
                brake,
                0.0
            );
            energy.speed_limit = PaddedVec2::new(
                WorkModule::get_param_float(boma, smash::hash40("battle_object"), smash::hash40("damage_speed_limit")),
                0.0
            );
        },
        _ => {}
    }
    true
}

#[cfg(feature = "dev-plugin")]
#[no_mangle]
pub unsafe extern "Rust" fn setup_stop(energy: &mut FighterKineticEnergyStop, reset_type: EnergyStopResetType, initial_speed: &PaddedVec2, unk: u64, boma: &mut BattleObjectModuleAccessor) -> bool {
    use EnergyStopResetType::*;

    if reset_type == AirLassoRewind {
        energy.reset_type = reset_type;
        return true;
    }

    energy.speed = PaddedVec2::zeros();
    energy.rot_speed = PaddedVec2::zeros();
    energy.accel = PaddedVec2::zeros();
    energy.speed_max = PaddedVec2::zeros();
    energy.speed_brake = PaddedVec2::zeros();
    energy.speed_limit = PaddedVec2::new(-1.0, -1.0);
    energy.reset_type = reset_type;
    energy.speed = *initial_speed;

    match reset_type {
        Ground | CatchCut | ItemSwingDash | ItemDashThrow => {
            let speed = energy.get_speed();
            let adjusted_speed = energy::KineticEnergy::adjust_speed_for_ground_normal(speed, boma);
            *speed = adjusted_speed;

            // get magnitude of speed vector
            let magnitude = (adjusted_speed.x.powi(2) + adjusted_speed.y.powi(2)).sqrt();

            energy._xBB = WorkModule::get_param_float(boma, smash::hash40("walk_speed_max"), 0) < magnitude;
        },
        DamageGround | GuardDamage | Run | RunBrake | CatchDash | ShieldRebound => {
            let speed = energy.get_speed();
            *speed = energy::KineticEnergy::adjust_speed_for_ground_normal(speed, boma);
        },
        DamageKnockBack => loop { // easier to follow if I structure this as a loop
            if StatusModule::situation_kind(boma) != *SITUATION_KIND_GROUND {
                break;
            }

            let damage_log = DamageModule::damage_log(boma);
            let object_id = *(damage_log as *const u32).add(0x84 / 0x4);
            let object = get_battle_object_from_id(object_id);
            if object.is_null() {
                println!("DamageKnockBack: object is null!");
                break;
            }

            let vtable_method: extern "C" fn(*mut BattleObject) -> bool = std::mem::transmute(**(object as *const *const u64));
            if vtable_method(object) || *(object as *const u8).add(0x3A) <= 3 {
                break;
            }

            let area_kind = JostleModule::area_kind(boma);
            if !AreaModule::is_exist_area_instance(boma, area_kind as i32) {
                break;
            }

            let area_kind = JostleModule::area_kind((*object).module_accessor);
            if !AreaModule::is_exist_area_instance((*object).module_accessor, area_kind as i32) {
                break;
            }

            let area_module = *((*object).module_accessor as *const u64).add(0xC0 / 0x8);
            let get_area: extern "C" fn(u64, i32) -> u64 = std::mem::transmute(*(*(area_module as *const *const u64)).add(0x118 / 0x8));
            let area = get_area(area_module, area_kind as i32);
            let our_pos = PostureModule::pos(boma);
            let their_pos = PostureModule::pos((*object).module_accessor);

            let does_model_have_joint = |boma: *mut BattleObjectModuleAccessor, hash: Hash40| {
                let model_module = *(boma as *const u64).add(0x78 / 0x8);
                let function: extern "C" fn(u64, Hash40) -> bool = std::mem::transmute(*(*(model_module as *const *const u64)).add(0x320 / 0x8));
                function(model_module, hash)
            };

            let (is_overlapping, other_pos) /* maybe */ = if (*our_pos).x >= (*their_pos).x {
                let x_pos = *(area as *const f32).add(0x50 / 0x4);
                let x_pos = if does_model_have_joint((*object).module_accessor, Hash40::new_raw(0x14d5b6ea53)) {
                    let mut pos = Vector3f { x: 0.0, y: 0.0, z: 0.0 };
                    ModelModule::joint_global_position((*object).module_accessor, Hash40::new_raw(0x14d5b6ea53), &mut pos, true);
                    pos.x.max(x_pos)
                } else {
                    x_pos
                };

                let x_pos = if does_model_have_joint((*object).module_accessor, Hash40::new_raw(0x142fb9d730)) {
                    let mut pos = Vector3f { x: 0.0, y: 0.0, z: 0.0 };
                    ModelModule::joint_global_position((*object).module_accessor, Hash40::new_raw(0x142fb9d730), &mut pos, true);
                    pos.x.max(x_pos)
                } else {
                    x_pos
                };

                ((*our_pos).x < x_pos, x_pos)
            } else {
                let x_pos = *(area as *const f32).add(0x40 / 0x4);
                let x_pos = if does_model_have_joint((*object).module_accessor, Hash40::new_raw(0x14d5b6ea53)) {
                    let mut pos = Vector3f { x: 0.0, y: 0.0, z: 0.0 };
                    ModelModule::joint_global_position((*object).module_accessor, Hash40::new_raw(0x14d5b6ea53), &mut pos, true);
                    pos.x.min(x_pos)
                } else {
                    x_pos
                };

                let x_pos = if does_model_have_joint((*object).module_accessor, Hash40::new_raw(0x142fb9d730)) {
                    let mut pos = Vector3f { x: 0.0, y: 0.0, z: 0.0 };
                    ModelModule::joint_global_position((*object).module_accessor, Hash40::new_raw(0x142fb9d730), &mut pos, true);
                    pos.x.min(x_pos)
                } else {
                    x_pos
                };

                (x_pos < (*our_pos).x, x_pos)
            };

            energy.elapsed_hitstop_frames = 0.0;
            energy.hitstop_frames = 0.0;
            energy._xAC = 0.0;
            energy._xB0 = 0.0;

            let overlap = if !is_overlapping {
                0.0
            } else {
                other_pos - (*our_pos).x
            };

            let hitstop_frames = *(damage_log as *const i32).add(0x4C / 4);

            let frame_rate = WorkModule::get_param_float(boma, smash::hash40("common"), smash::hash40("damage_knock_back_hitstop_frame_rate"));
            
            energy.hitstop_frames = (frame_rate * 0.01 * hitstop_frames as f32).max(1.0);
            energy._xB0 = overlap;
            let speed_rate = WorkModule::get_param_float(boma, smash::hash40("common"), smash::hash40("damage_knock_back_speed_x_rate"));
            energy.speed = PaddedVec2::new(
                overlap * speed_rate * 0.01,
                0.0
            );            

            break;
        },
        AirLassoHang => {
            energy._x90 = *initial_speed;
        },
        EscapeAirSlide => {
            let energy_speed = *energy.get_speed();
            energy._x90 = *initial_speed;
            let speed = WorkModule::get_param_float(boma, smash::hash40("escape_air_slide_speed"), 0);
            let accel = WorkModule::get_param_float(boma, smash::hash40("escape_air_slide_accel"), 0);
            energy.speed = PaddedVec2::new(energy_speed.x * speed, energy_speed.y * speed);
            energy.speed_brake = PaddedVec2::new((energy_speed.x * accel).abs(), (energy_speed.y * accel).abs());
            energy.speed_limit = PaddedVec2::new(-1.0, -1.0);
        }
        _ => {},
    }

    energy.initialize(boma);
    energy._xBA = false;
    energy._xB8 = 0;
    energy._xB4 = 0;

    true
}

#[cfg(not(feature = "dev-plugin"))]
#[skyline::hook(offset = 0x6d6630)]
unsafe fn update_stop_hook(energy: &mut FighterKineticEnergyStop, boma: &mut BattleObjectModuleAccessor) {
    extern "Rust" {
        fn update_stop(energy: &mut FighterKineticEnergyStop, boma: &mut BattleObjectModuleAccessor) -> bool;
    }

    if super::SHOULD_RUN {
        if !update_stop(energy, boma) {
            call_original!(energy, boma)
        }
    }
}

#[cfg(not(feature = "dev-plugin"))]
#[skyline::hook(offset = 0x6d80e0)]
unsafe fn initialize_stop_hook(energy: &mut FighterKineticEnergyStop, boma: &mut BattleObjectModuleAccessor) {
    extern "Rust" {
        fn initialize_stop(energy: &mut FighterKineticEnergyStop, boma: &mut BattleObjectModuleAccessor) -> bool;
    }

    if super::SHOULD_RUN {
        if !initialize_stop(energy, boma) {
            call_original!(energy, boma)
        }
    }
}

#[cfg(not(feature = "dev-plugin"))]
#[skyline::hook(offset = 0x6d8540)]
unsafe fn setup_stop_hook(energy: &mut FighterKineticEnergyStop, reset_type: EnergyStopResetType, initial_speed: &PaddedVec2, unk: u64, boma: &mut BattleObjectModuleAccessor) {
    extern "Rust" {
        fn setup_stop(energy: &mut FighterKineticEnergyStop, reset_type: EnergyStopResetType, initial_speed: &PaddedVec2, unk: u64, boma: &mut BattleObjectModuleAccessor) -> bool;
    }

    if super::SHOULD_RUN {
        if !setup_stop(energy, reset_type, initial_speed, unk, boma) {
            call_original!(energy, reset_type, initial_speed, unk, boma)
        }
    }
    
}

pub fn install() {
    #[cfg(not(feature = "dev-plugin"))]
    {
        skyline::install_hooks!(
            update_stop_hook,
            initialize_stop_hook,
            setup_stop_hook
        );
    }
}