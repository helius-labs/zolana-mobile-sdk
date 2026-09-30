import 'dart:io';

import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart'
    show ExternalLibrary;

import 'rust/frb_generated.dart';

/// Load the native library. Call once, before any other API of this package.
///
/// On Android the Rust library ships as `libmopro_flutter_bindings.so`, which
/// is where the bridge looks by default. On iOS and macOS the pod force-loads
/// the Rust static library into this plugin's framework, so there is no
/// `mopro_flutter_bindings.framework` for the default loader to find: open
/// the plugin framework instead, or, when the app links pods statically and
/// the code is in its own binary, the process.
Future<void> initZolanaMobile() {
  if (!Platform.isIOS && !Platform.isMacOS) return RustLib.init();
  ExternalLibrary library;
  try {
    library = ExternalLibrary.open('zolana_mobile.framework/zolana_mobile');
  } catch (_) {
    library = ExternalLibrary.process(iKnowHowToUseIt: true);
  }
  return RustLib.init(externalLibrary: library);
}
