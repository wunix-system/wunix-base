pub mod datetime;
pub mod disk;
pub mod finish;
pub mod fs;
pub mod network;
pub mod partition;
pub mod progress;
pub mod user;
pub mod welcome;

pub use datetime::screen_datetime_and_timezone;
pub use disk::screen_select_disk;
pub use finish::{screen_error, screen_finish, FinishChoice};
pub use fs::screen_select_fs;
pub use network::{screen_network_config, screen_select_adapter};
pub use partition::{
    screen_partition_confirm, screen_partition_manual, screen_partition_mode, PartitionMode,
};
pub use progress::{
    begin_install_progress_screen, begin_partition_process_screen, update_install_progress,
    update_partition_step,
};
pub use user::screen_user_and_hostname;
pub use welcome::{screen_boot_menu, screen_welcome_info, WelcomeChoice};
