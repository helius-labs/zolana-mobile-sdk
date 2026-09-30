/// This is copied from Cargokit (which is the official way to use it currently)
/// Details: https://fzyzcjy.github.io/flutter_rust_bridge/manual/integrate/builtin

import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:collection/collection.dart';
import 'package:convert/convert.dart';
import 'package:crypto/crypto.dart';
import 'package:path/path.dart' as path;
import 'package:toml/toml.dart';

class CrateHash {
  /// Computes a hash uniquely identifying crate content. This takes into account
  /// the path and content of every file of the crate and of its path
  /// dependencies, skipping `target` and hidden entries.
  ///
  /// If [tempStorage] is provided, computed hash is stored in a file in that directory
  /// and reused on subsequent calls if the crate content hasn't changed.
  static String compute(String manifestDir, {String? tempStorage}) {
    return CrateHash._(
      manifestDir: manifestDir,
      tempStorage: tempStorage,
    )._compute();
  }

  CrateHash._({
    required this.manifestDir,
    required this.tempStorage,
  });

  String _compute() {
    final files = getFiles();
    final tempStorage = this.tempStorage;
    if (tempStorage != null) {
      final quickHash = _computeQuickHash(files);
      final quickHashFolder = Directory(path.join(tempStorage, 'crate_hash'));
      quickHashFolder.createSync(recursive: true);
      final quickHashFile = File(path.join(quickHashFolder.path, quickHash));
      if (quickHashFile.existsSync()) {
        return quickHashFile.readAsStringSync();
      }
      final hash = _computeHash(files);
      quickHashFile.writeAsStringSync(hash);
      return hash;
    } else {
      return _computeHash(files);
    }
  }

  /// Computes a quick hash based on files stat (without reading contents). This
  /// is used to cache the real hash, which is slower to compute since it involves
  /// reading every single file.
  String _computeQuickHash(List<File> files) {
    final output = AccumulatorSink<Digest>();
    final input = sha256.startChunkedConversion(output);

    final data = ByteData(8);
    for (final file in files) {
      input.add(utf8.encode(file.path));
      final stat = file.statSync();
      data.setUint64(0, stat.size);
      input.add(data.buffer.asUint8List());
      data.setUint64(0, stat.modified.millisecondsSinceEpoch);
      input.add(data.buffer.asUint8List());
    }

    input.close();
    return base64Url.encode(output.events.single.bytes);
  }

  String _computeHash(List<File> files) {
    final output = AccumulatorSink<Digest>();
    final input = sha256.startChunkedConversion(output);

    void addTextFile(File file) {
      // Files are hashed by lines in case we're dealing with github checkout
      // that auto-converts line endings.
      input.add(utf8.encode(_relativePath(file)));
      input.add([0]);
      final data = utf8.decode(file.readAsBytesSync(), allowMalformed: true);
      for (final line in LineSplitter().convert(data)) {
        input.add(utf8.encode(line));
      }
    }

    for (final file in files) {
      addTextFile(file);
    }

    input.close();
    final res = output.events.single;

    // Truncate to 128bits.
    final hash = res.bytes.sublist(0, 16);
    return hex.encode(hash);
  }

  String get _root => path.normalize(path.absolute(manifestDir));

  /// Path relative to the crate, with `/` separators on every host.
  String _relativePath(File file) =>
      path.posix.joinAll(path.split(path.relative(file.path, from: _root)));

  List<File> getFiles() {
    final files = <File>[];
    for (final dir in _crateDirs()) {
      _collectFiles(Directory(dir), files);
    }
    files.sortBy(_relativePath);
    return files;
  }

  /// The crate directory and the directories of its path dependencies.
  Set<String> _crateDirs() {
    final dirs = <String>{};
    void visit(String dir) {
      if (!dirs.add(dir)) return;
      for (final dependency in _pathDependencies(dir)) {
        visit(path.normalize(path.join(dir, dependency)));
      }
    }

    visit(_root);
    return dirs;
  }

  static Iterable<String> _pathDependencies(String dir) {
    final manifest = File(path.join(dir, 'Cargo.toml')).readAsStringSync();
    final toml = TomlDocument.parse(manifest).toMap();
    return ['dependencies', 'build-dependencies']
        .map((section) => toml[section])
        .whereType<Map>()
        .expand((dependencies) => dependencies.values)
        .whereType<Map>()
        .map((dependency) => dependency['path'])
        .whereType<String>();
  }

  static void _collectFiles(Directory dir, List<File> files) {
    for (final entity in dir.listSync(followLinks: false)) {
      final name = path.basename(entity.path);
      if (name.startsWith('.') || name == 'target') continue;
      if (entity is Directory) {
        _collectFiles(entity, files);
      } else if (entity is File) {
        files.add(entity);
      }
    }
  }

  final String manifestDir;
  final String? tempStorage;
}
