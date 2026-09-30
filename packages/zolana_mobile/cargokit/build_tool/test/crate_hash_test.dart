import 'dart:io';

import 'package:build_tool/src/crate_hash.dart';
import 'package:path/path.dart' as path;
import 'package:test/test.dart';

void main() {
  late Directory root;

  setUp(() => root = Directory.systemTemp.createTempSync('crate_hash_'));
  tearDown(() => root.deleteSync(recursive: true));

  void write(String relative, String contents) {
    File(path.join(root.path, relative))
      ..createSync(recursive: true)
      ..writeAsStringSync(contents);
  }

  String hash(String crate) => CrateHash.compute(path.join(root.path, crate));

  test('covers path dependencies and skips target and hidden entries', () {
    write('a/Cargo.toml', '''
[package]
name = "a"

[dependencies.b]
path = "../b"
''');
    write('a/src/lib.rs', '');
    write('b/Cargo.toml', '''
[package]
name = "b"

[build-dependencies]
c = { path = "../c" }
''');
    write('c/Cargo.toml', '[package]\nname = "c"\n');
    write('c/go/main.go', 'package main\n');
    final initial = hash('a');

    write('b/target/out', 'build output');
    write('c/.cache/out', 'hidden');
    expect(hash('a'), initial);

    write('c/go/main.go', 'package main\n\nfunc main() {}\n');
    expect(hash('a'), isNot(initial));

    write('c/go/main.go', 'package main\n');
    write('c/go/other.go', '');
    expect(hash('a'), isNot(initial));
  });

  test('does not depend on the location or line endings', () {
    write('x/a/Cargo.toml', '[package]\nname = "a"\n');
    write('x/a/src/lib.rs', 'fn a() {}\n');
    write('y/z/a/Cargo.toml', '[package]\r\nname = "a"\r\n');
    write('y/z/a/src/lib.rs', 'fn a() {}\r\n');
    expect(hash('x/a'), hash('y/z/a'));
  });
}
