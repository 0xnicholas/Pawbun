//! FileLoader bridge tools — implemented in Task 2.5.

use pawbun_toolkit::ToolKit;
use pawbun_files::DefaultFileLoader;

/// Register bridge tools derived from the FileLoader.
///
/// Automatically adds `file_read` and `file_list` tools if not already
/// present in the toolkit (user-registered tools take priority).
pub(crate) fn register_bridge_tools(_toolkit: &mut ToolKit, _loader: DefaultFileLoader) {
    // Implemented in Task 2.5
}
