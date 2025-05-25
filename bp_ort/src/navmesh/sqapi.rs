use rrplug::{
    high::squirrel::{UserData, UserDataRef},
    prelude::*,
};

use crate::navmesh::{
    navigation::{Navigation, NavigationError},
    Hull, RECAST_DETOUR,
};

pub fn navigation_register_sq_functions() {
    register_sq_functions(navigation_new);
    register_sq_functions(navigation_find_path);
    register_sq_functions(navigation_get_all_points);
    register_sq_functions(navigation_next_point);
    register_sq_functions(navigation_random_point);
}

#[rrplug::sqfunction(VM = "SERVER", ExportName = "NavigationCreate")]
fn navigation_new(hull: Hull) -> Option<UserData<Navigation>> {
    Some(UserData::new(Navigation::new(hull)?))
}

#[rrplug::sqfunction(VM = "SERVER", ExportName = "NavigationFindPath")]
fn navigation_find_path(
    mut nav: UserDataRef<Navigation>,
    start: Vector3,
    end: Vector3,
) -> Result<(), NavigationError> {
    nav.new_path(start, end, RECAST_DETOUR.wait())
}

#[rrplug::sqfunction(VM = "SERVER", ExportName = "NavigationGetAllPoints")]
fn navigation_get_all_points(nav: UserDataRef<Navigation>) -> Vec<Vector3> {
    nav.path_points.clone()
}

#[rrplug::sqfunction(VM = "SERVER", ExportName = "NavigationNextPoint")]
fn navigation_next_point(mut nav: UserDataRef<Navigation>) -> Option<Vector3> {
    nav.next_point()
}

#[rrplug::sqfunction(VM = "SERVER", ExportName = "NavigationRandomPoint")]
fn navigation_random_point(
    mut nav: UserDataRef<Navigation>,
    center: Vector3,
    radius: f32,
) -> Option<Vector3> {
    nav.random_point_around(center, radius, None)
}
