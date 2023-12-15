// cargo build --package gm_kinect && cp target/debug/gm_kinect.dll "D:\Steam\steamapps\common\GarrysMod\garrysmod\lua\bin\gmcl_kinect_win64.dll" && cp target/debug/gm_kinect.dll "D:\Steam\steamapps\common\GarrysMod\garrysmod\lua\bin\gmsv_kinect_win64.dll"
#![feature(array_chunks)]
#![feature(c_unwind)]
#![feature(option_get_or_insert_default)]
#![feature(thread_id_value)]

#[macro_use]
extern crate gmod;

use kinect::Kinect;
use std::{
    borrow::Cow,
    cell::Cell,
    ffi::OsString,
    fs::OpenOptions,
    mem::{size_of, ManuallyDrop},
    path::Path,
};

static mut KINECT: Option<KinectState> = None;

const MMAP_FILE_SIZE: u64 = (size_of::<u8>()
    + size_of::<u8>()
    + size_of::<u16>()
    + (size_of::<[f32; 3]>() * kinect::SKELETON_BONE_COUNT)) as u64;

const MMAP_KINECT_SKELETON_NONE: u8 = 0;
const MMAP_KINECT_SKELETON_TRACKED: u8 = 1;

const MMAP_SHUTDOWN: usize = 0;
const MMAP_SYNC: std::ops::Range<usize> = 1..3;
const MMAP_SKELETON: usize = 3;
const MMAP_SKELETON_BONES: std::ops::RangeFrom<usize> = 4..;

struct KinectState {
    mmap: memmap::MmapMut,
    skeleton: Option<[[f32; 3]; kinect::SKELETON_BONE_COUNT]>,
    kind: KinectStateKind,
}
impl KinectState {
    fn new(lua: gmod::lua::State) -> Result<Self, std::io::Error> {
        // We're a client if garrysmod/cache/gm_kinect/klient_pid.dat exists
        let mmap_name = OsString::from(format!("kinect_{}", std::process::id()));
        let client = 'client: {
            if let Ok(dir) = std::fs::read_dir("garrysmod/cache/gm_kinect") {
                for entry in dir.flatten() {
                    let entry = entry.path();
                    if entry.file_name() == Some(mmap_name.as_os_str()) {
                        break 'client true;
                    }
                }
            }
            break 'client false;
        };
        if !client {
            // Clean up old mmaps
            std::fs::remove_dir_all("garrysmod/cache/gm_kinect").ok();
        }

        std::fs::create_dir_all("garrysmod/cache/gm_kinect")?;

        let mmap_path = Path::new("garrysmod/cache/gm_kinect").join(mmap_name);

        let f = OpenOptions::new()
            .write(true)
            .read(true)
            .truncate(false)
            .create(true)
            .open(mmap_path)?;

        f.set_len(MMAP_FILE_SIZE)?;

        let mut mmap = unsafe { memmap::MmapMut::map_mut(&f)? };

        if client {
            unsafe {
                lua.get_global(lua_string!("print"));
                lua.push_string("gm_kinect: started client");
                lua.call(1, 0);
            }

            let mut client = Self {
                mmap,
                skeleton: None,
                kind: KinectStateKind::Client { sync: None },
            };

            client.update(lua);

            Ok(client)
        } else {
            unsafe {
                lua.get_global(lua_string!("print"));
                lua.push_string("gm_kinect: started server");
                lua.call(1, 0);
            }

            let inner = Kinect::new()?;

            mmap[0..4].copy_from_slice(&[0, 0, 0, 0]);
            mmap.flush_range(0, 4).ok();

            Ok(Self {
                mmap,
                skeleton: None,
                kind: KinectStateKind::Server {
                    inner: ManuallyDrop::new(inner),
                    sync: 0,
                },
            })
        }
    }

    fn update(&mut self, lua: gmod::lua::State) {
        match &mut self.kind {
            KinectStateKind::Server { inner, sync } => {
                let mut last_update = None;
                while let Some(update) = inner.poll() {
                    last_update = Some(update);
                }
                let Some(last_update) = last_update else {
                    return;
                };

                *sync = sync.wrapping_add(1);
                self.mmap[MMAP_SYNC].copy_from_slice(&u16::to_ne_bytes(*sync));

                unsafe {
                    lua.get_global(lua_string!("print"));
                    lua.push_string("gm_kinect: server update");
                    lua.call(1, 0);
                }

                if let Some(kinect::Skeleton::Tracked(pos)) = last_update.pos() {
                    self.mmap[MMAP_SKELETON] = MMAP_KINECT_SKELETON_TRACKED;

                    let skeleton = self.skeleton.get_or_insert_default();

                    for ((vec, mmap), skeleton) in pos
                        .raw_bones()
                        .iter()
                        .zip(
                            self.mmap[MMAP_SKELETON_BONES]
                                .array_chunks_mut::<{ size_of::<[f32; 3]>() }>(),
                        )
                        .zip(skeleton.iter_mut())
                    {
                        mmap[0..4].copy_from_slice(&f32::to_ne_bytes(vec.x));
                        mmap[4..8].copy_from_slice(&f32::to_ne_bytes(vec.y));
                        mmap[8..12].copy_from_slice(&f32::to_ne_bytes(vec.z));

                        *skeleton = [vec.x, vec.y, vec.z];
                    }

                    self.mmap.flush_range(1, (MMAP_FILE_SIZE - 1) as _).ok();
                } else {
                    self.mmap[MMAP_SKELETON] = MMAP_KINECT_SKELETON_NONE;
                    self.mmap.flush_range(1, 3).ok();

                    self.skeleton = None;
                }
            }

            KinectStateKind::Client { sync } => {
                let shutdown = self.mmap[MMAP_SHUTDOWN];
                if shutdown == 1 {
                    unsafe {
                        lua.get_global(lua_string!("print"));
                        lua.push_string("gm_kinect: trying to promote to server");
                        lua.call(1, 0);
                    }

                    // Promote to server
                    if let Ok(inner) = Kinect::new() {
                        if core::mem::replace(&mut self.mmap[MMAP_SHUTDOWN], 0) != 1 {
                            return self.update(lua);
                        }

                        if self.mmap.flush_range(0, 1).is_ok() {
                            self.kind = KinectStateKind::Server {
                                inner: ManuallyDrop::new(inner),
                                sync: sync.unwrap_or(0),
                            };

                            unsafe {
                                lua.get_global(lua_string!("print"));
                                lua.push_string("gm_kinect: promoted to server");
                                lua.call(1, 0);
                            }

                            return self.update(lua);
                        }
                    }
                    return;
                }

                let new_sync = Some(u16::from_ne_bytes(self.mmap[MMAP_SYNC].try_into().unwrap()));
                if new_sync == core::mem::replace(sync, new_sync) {
                    // No changes
                    return;
                }

                unsafe {
                    lua.get_global(lua_string!("print"));
                    lua.push_string("gm_kinect: client update");
                    lua.call(1, 0);
                }

                match self.mmap[MMAP_SKELETON] {
                    MMAP_KINECT_SKELETON_NONE => {
                        self.skeleton = None;
                    }

                    MMAP_KINECT_SKELETON_TRACKED => {
                        let skeleton = self.skeleton.get_or_insert_default();
                        for (bone, skeleton) in self.mmap[MMAP_SKELETON_BONES]
                            .array_chunks::<{ size_of::<[f32; 3]>() }>()
                            .zip(skeleton.iter_mut())
                        {
                            *skeleton = [
                                f32::from_ne_bytes(bone[0..4].try_into().unwrap()),
                                f32::from_ne_bytes(bone[4..8].try_into().unwrap()),
                                f32::from_ne_bytes(bone[8..12].try_into().unwrap()),
                            ];
                        }
                    }

                    _ => unreachable!(),
                }
            }
        }
    }
}
impl Drop for KinectState {
    fn drop(&mut self) {
        if let KinectStateKind::Server { inner, .. } = &mut self.kind {
            // Shut down Kinect
            unsafe { ManuallyDrop::drop(inner) };

            // Mark shutdown byte
            self.mmap[MMAP_SHUTDOWN] = 1;
            self.mmap.flush_range(0, 1).ok();
        }
    }
}

enum KinectStateKind {
    Server {
        inner: ManuallyDrop<Kinect>,
        sync: u16,
    },

    Client {
        sync: Option<u16>,
    },
}

#[lua_function]
unsafe fn poll(_lua: gmod::lua::State) {
    if let Some(kinect) = &mut KINECT {
        kinect.update(_lua);
    }
}

#[lua_function]
unsafe fn start(lua: gmod::lua::State) -> i32 {
    lua.get_global(lua_string!("print"));
    lua.push_string("gm_kinect: motionsensor.Start()");
    lua.call(1, 0);

    lua.push_boolean(if KINECT.is_none() {
        match KinectState::new(lua) {
            Ok(kinect) => {
                KINECT = Some(kinect);

                lua.get_global(lua_string!("hook"));
                lua.get_field(-1, lua_string!("Add"));
                lua.push_string("Think");
                lua.push_string("gm_kinect");
                lua.push_function(poll);
                lua.call(3, 0);
                lua.pop();

                true
            }
            Err(err) => {
                lua.get_global(lua_string!("ErrorNoHalt"));
                lua.push_string(&format!("Kinect error: {err:?}\n"));
                lua.call(1, 0);

                false
            }
        }
    } else {
        false
    });

    1
}

#[lua_function]
unsafe fn stop(lua: gmod::lua::State) -> i32 {
    lua.get_global(lua_string!("print"));
    lua.push_string("gm_kinect: motionsensor.Stop()");
    lua.call(1, 0);

    lua.get_global(lua_string!("hook"));
    lua.get_field(-1, lua_string!("Remove"));
    lua.push_string("Think");
    lua.push_string("gm_kinect");
    lua.call(2, 0);
    lua.pop();

    KINECT = None;

    0
}

#[lua_function]
unsafe fn is_active(lua: gmod::lua::State) -> i32 {
    lua.push_boolean(KINECT.is_some());
    1
}

#[lua_function]
unsafe fn is_available(lua: gmod::lua::State) -> i32 {
    // FIXME
    lua.push_boolean(true);
    1
}

#[lua_function]
unsafe fn get_table(lua: gmod::lua::State) -> i32 {
    lua.create_table(kinect::SKELETON_BONE_COUNT as _, 0);

    if let Some(kinect) = &mut KINECT {
        kinect.update(lua);

        if let Some(skeleton) = &kinect.skeleton {
            // TODO replace with gmod vector

            lua.get_global(lua_string!("Vector"));

            for (i, pos) in skeleton.iter().enumerate() {
                lua.push_value(-1);
                lua.push_number(pos[0] as _);
                lua.push_number(pos[1] as _);
                lua.push_number(pos[2] as _);
                lua.call(3, 1);
                lua.raw_seti(-3, i as _);
            }

            lua.pop();

            return 1;
        }
    }

    // Push zeroed table
    lua.get_global(lua_string!("vector_origin"));
    for i in 0..kinect::SKELETON_BONE_COUNT as i32 {
        lua.push_value(-1);
        lua.raw_seti(-3, i);
    }
    lua.pop();

    1
}

#[lua_function]
unsafe fn player_motion_sensor_pos(lua: gmod::lua::State) -> i32 {
    let pos = if let Some(kinect) = &mut KINECT {
        kinect.update(lua);

        if let Some(skeleton) = &mut kinect.skeleton {
            usize::try_from(lua.to_integer(2))
                .ok()
                .and_then(|idx| skeleton.get(idx))
        } else {
            None
        }
    } else {
        None
    };

    // TODO other players
    // TOOD use gmod vector
    lua.get_global(lua_string!("Vector"));
    if let Some(pos) = pos {
        lua.push_number(pos[0] as _);
        lua.push_number(pos[1] as _);
        lua.push_number(pos[2] as _);
    } else {
        lua.push_integer(0);
        lua.push_integer(0);
        lua.push_integer(0);
    }
    lua.call(3, 1);

    1
}

#[lua_function]
unsafe fn get_colour_material(lua: gmod::lua::State) -> i32 {
    // TODO
    lua.get_global(lua_string!("Material"));
    lua.push_string("pp/colour");
    lua.call(1, 1);
    1
}

#[gmod13_open]
fn gmod13_open(lua: gmod::lua::State) {
    thread_local! {
        static LUA_STATE: Cell<Option<gmod::lua::State>> = Cell::new(None);
    }

    LUA_STATE.set(Some(lua));

    std::panic::set_hook(Box::new(move |panic| {
        let path = if let Some(lua) = LUA_STATE.get() {
            unsafe {
                lua.get_global(lua_string!("ErrorNoHalt"));
                lua.push_string(&format!("Kinect panic: {:#?}\n", panic));
                lua.call(1, 0);
            }
            Cow::Borrowed("gm_kinect_panic.txt")
        } else {
            Cow::Owned(format!(
                "gm_kinect_panic_{}.txt",
                std::thread::current().id().as_u64()
            ))
        };

        std::fs::write(path.as_ref(), format!("{:#?}", panic)).ok();
    }));

    unsafe {
        lua.get_global(lua_string!("print"));
        lua.push_string("gm_kinect loaded!");
        lua.call(1, 0);

        lua.get_global(lua_string!("motionsensor"));
        if lua.is_nil(-1) {
            lua.create_table(0, 0);
            lua.set_global(lua_string!("motionsensor"));
            lua.get_global(lua_string!("motionsensor"));
        }

        lua.push_string("Start");
        lua.push_function(start);
        lua.set_table(-3);

        lua.push_string("Stop");
        lua.push_function(stop);
        lua.set_table(-3);

        lua.push_string("IsActive");
        lua.push_function(is_active);
        lua.set_table(-3);

        lua.push_string("IsAvailable");
        lua.push_function(is_available);
        lua.set_table(-3);

        lua.push_string("GetTable");
        lua.push_function(get_table);
        lua.set_table(-3);

        lua.push_string("GetColourMaterial");
        lua.push_function(get_colour_material);
        lua.set_table(-3);

        lua.pop();

        lua.get_global(lua_string!("FindMetaTable"));
        lua.push_string("Player");
        lua.call(1, 1);

        if !lua.is_nil(-1) {
            lua.push_string("MotionSensorPos");
            lua.push_function(player_motion_sensor_pos);
            lua.set_table(-3);
        }

        lua.pop();
    }
}

#[gmod13_close]
fn gmod13_close(_lua: gmod::lua::State) {
    unsafe { KINECT = None };
}
