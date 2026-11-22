pub mod panel;

pub const USAGE: &str = "Usage:\n  fabriccraft-cli surfaces <mesh.obj> <mm|cm|m|in>\n  fabriccraft-cli seam-vertices <mesh.obj> <mm|cm|m|in> [--exclude-surfaces NUMBER,...]\n  fabriccraft-cli panel <mesh.obj> <mm|cm|m|in> <pattern.svg> [--surface NUMBER] [--fabric-mm WIDTH LENGTH]\n  fabriccraft-cli develop <mesh.obj> <mm|cm|m|in> <pattern.svg> [--exclude-surfaces NUMBER,...] [--open-tube | --seam VERTEX,VERTEX,...] [--fabric-mm WIDTH LENGTH]";
