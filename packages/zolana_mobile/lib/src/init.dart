import 'rust/frb_generated.dart';

/// Load the native library. Call once, before any other API of this package.
///
/// The bridge's default loader finds it: `libmopro_flutter_bindings.so` on
/// Android and `mopro_flutter_bindings.framework`, which the package's build
/// hook provides, on iOS.
Future<void> initZolanaMobile() => RustLib.init();
