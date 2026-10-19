import 'dart:io';

import 'package:code_assets/code_assets.dart';
import 'package:hooks/hooks.dart';

/// The oldest iOS the library runs on, and Flutter's. The precompiled binaries
/// are built for it (`IPHONEOS_DEPLOYMENT_TARGET` in the precompile workflow).
const _minimumIOS = 15;

/// The Rust library, `[package] name` in rust/Cargo.toml.
const _library = 'mopro_flutter_bindings';

/// Provides the iOS native library: a dynamic library that Flutter bundles as
/// `mopro_flutter_bindings.framework`, where the bridge's default loader finds
/// it. Neither CocoaPods nor Swift Package Manager builds it, so an app builds
/// with either of them.
///
/// Cargokit provides the Rust static library with the Go prover archive in
/// it: the signed precompiled library of the crate hash, or a build of the
/// crate when the release has none. This hook links it into the dynamic
/// library. Android builds the library through the Gradle plugin of cargokit
/// and has nothing to do here.
void main(List<String> args) async {
  await build(args, (input, output) async {
    if (!input.config.buildCodeAssets) return;
    final code = input.config.code;
    if (code.targetOS != OS.iOS) return;

    final ios = code.iOS;
    final simulator = ios.targetSdk == IOSSdk.iPhoneSimulator;
    final sdk = simulator ? 'iphonesimulator' : 'iphoneos';
    final arch = switch (code.targetArchitecture) {
      Architecture.arm64 => 'arm64',
      Architecture.x64 => 'x86_64',
      final other => throw BuildError(message: 'iOS on $other is unsupported'),
    };
    final root = input.packageRoot;
    final shared = input.outputDirectoryShared;
    final staticDir = input.outputDirectory.resolve('static/');

    await _run(
      'bash',
      [root.resolve('cargokit/run_build_tool.sh').toFilePath(), 'build-pod'],
      {
        'PATH': _toolPath(),
        'CARGOKIT_DARWIN_PLATFORM_NAME': sdk,
        'CARGOKIT_DARWIN_ARCHS': arch,
        'CARGOKIT_CONFIGURATION': 'Release',
        'CARGOKIT_MANIFEST_DIR': root.resolve('rust/').toFilePath(),
        'CARGOKIT_TARGET_TEMP_DIR': shared.resolve('cargokit/').toFilePath(),
        'CARGOKIT_OUTPUT_DIR': staticDir.toFilePath(),
        'CARGOKIT_TOOL_TEMP_DIR': shared.resolve('build_tool/').toFilePath(),
        // Cargokit looks for the app's cargokit_options.yaml from here upwards;
        // the hook output is inside the app's .dart_tool directory.
        'CARGOKIT_ROOT_PROJECT_DIR': shared.toFilePath(),
        // A build from source compiles its C code for this iOS, as the
        // precompiled binaries are.
        'IPHONEOS_DEPLOYMENT_TARGET': '$_minimumIOS.0',
      },
    );

    // Flutter writes the iOS version it gives the hook into the framework's
    // Info.plist as MinimumOSVersion, and App Store Connect rejects a
    // framework whose binary needs a newer iOS (ITMS-90208), so the library
    // is linked for that version. The version is a constant of the Flutter
    // release, not the app's deployment target: 13 before Flutter 3.47.4, 15
    // since. Below 15 the linker warns that the objects need iOS 15; Flutter
    // itself needs iOS 15, so no app runs the library on an older iOS.
    final dylib = input.outputDirectory.resolve('lib$_library.dylib');
    await _run('xcrun', [
      '--sdk',
      sdk,
      'clang',
      '-dynamiclib',
      '-target',
      '$arch-apple-ios${ios.targetVersion}.0${simulator ? '-simulator' : ''}',
      '-install_name',
      '@rpath/lib$_library.dylib',
      '-Wl,-force_load,${staticDir.resolve('lib$_library.a').toFilePath()}',
      // What the Go runtime of the prover needs.
      '-framework',
      'CoreFoundation',
      '-framework',
      'Security',
      '-lresolv',
      '-lc++',
      // What zopro's Metal driver needs.
      '-framework',
      'Foundation',
      '-framework',
      'Metal',
      '-o',
      dylib.toFilePath(),
    ]);

    output.assets.code.add(
      CodeAsset(
        package: input.packageName,
        name: _library,
        linkMode: DynamicLoadingBundled(),
        file: dylib,
      ),
    );
    output.dependencies.addAll([
      for (final dir in ['rust/', 'native/', 'cargokit/'])
        ..._sources(Directory.fromUri(root.resolve(dir))),
      ?_options(Directory.fromUri(shared)),
    ]);
  });
}

/// The app's cargokit_options.yaml, found as cargokit finds it. A file added
/// later takes effect after `flutter clean`.
Uri? _options(Directory dir) {
  for (; dir.parent.path != dir.path; dir = dir.parent) {
    final file = File.fromUri(dir.uri.resolve('cargokit_options.yaml'));
    if (file.existsSync()) return file.uri;
  }
  return null;
}

/// `PATH` for cargokit: first the Dart SDK running this hook, for cargokit's
/// build tool, and none of Xcode's directories, whose tools break the build
/// of that tool.
String _toolPath() => [
  File(Platform.resolvedExecutable).parent.path,
  ...?Platform.environment['PATH']
      ?.split(':')
      .where((entry) => !entry.contains('Contents/Developer/')),
].join(':');

/// The files whose change changes the library: those the crate hash covers,
/// and cargokit.
Iterable<Uri> _sources(Directory dir) sync* {
  for (final entity in dir.listSync(followLinks: false)) {
    final name = entity.uri.pathSegments.lastWhere((s) => s.isNotEmpty);
    if (name.startsWith('.') || name == 'target') continue;
    if (entity is Directory) {
      yield* _sources(entity);
    } else if (entity is File) {
      yield entity.uri;
    }
  }
}

Future<void> _run(
  String executable,
  List<String> arguments, [
  Map<String, String>? environment,
]) async {
  final result = await Process.run(
    executable,
    arguments,
    environment: environment,
  );
  stdout.write(result.stdout);
  stderr.write(result.stderr);
  if (result.exitCode != 0) {
    throw BuildError(
      message:
          '$executable ${arguments.join(' ')} exited with ${result.exitCode}:'
          '\n${result.stdout}\n${result.stderr}',
    );
  }
}
