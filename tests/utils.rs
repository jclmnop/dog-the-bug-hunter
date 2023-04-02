use std::process::{Command, Stdio};
use tracing::info;

pub struct DockerGuard<'a> {
    pub docker_dir: &'a str,
}

impl<'a> Drop for DockerGuard<'a> {
    fn drop(&mut self) {
        info!("Running docker-compose down (in drop)");
        let mut docker_down = Command::new("docker-compose")
            .arg("down")
            .current_dir(self.docker_dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("failed to run docker-compose down");
        docker_down.wait().expect("failed to wait on docker-compose down");
    }
}

pub fn start_docker<'a>() -> DockerGuard<'a> {
    info!("Running docker-compose up");
    // Set the working directory to the 'tests/docker' folder
    let docker_dir = "./tests/docker";

    // Run the docker-compose up -d command
    let mut docker_up = Command::new("docker-compose")
        .args(&["up", "-d"])
        .current_dir(docker_dir)
        .stdout(Stdio::null())
        .spawn()
        .expect("failed to run docker-compose up");
    docker_up.wait().expect("failed to wait on docker-compose up");

    DockerGuard { docker_dir }
}

pub fn start_logger() {
    let _ = env_logger::builder().filter_level(tracing::log::LevelFilter::Info).try_init();
}
