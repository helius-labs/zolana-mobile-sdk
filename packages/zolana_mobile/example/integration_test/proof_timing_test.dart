// Times proofs on this device as the wallet proves a spend, for the proof
// timing report (README, "Proof timing"): each shape's key through the
// wallet's key store, downloaded when missing, loaded when not loaded, the
// proof verified. It runs in release builds of the bench app
// (scripts/test-consumer.sh bench-apk, bench-ipa), twice on the same
// install for the four rows of the report: a run with no keys on the device
// is the fresh one (first run, including download; warm), and a run that
// finds them, in a new process, the restart (first run after restart;
// repeated). `ZOLANA_TIMING_RUN=both` makes the restart rows after the
// fresh ones in one process, with the wallet closed and reopened between,
// for devices that allow one test run per install (iOS on a device farm):
// the keys load again from the device, the process does not start again.
//
// A shape is `<inputs>x<outputs>` for a confidential transfer or
// `merge-<inputs>x1` for a merge; ZOLANA_TIMING_SHAPES narrows them.
//
// Every proof prints one `ZOLANA_TIMING {json}` line; both runs' lines go
// through scripts/proof-timing-report.py. The fixture requests come from
// assets/proving, where scripts/stage-demo-assets.sh copies them.
import 'dart:convert';
import 'dart:io';

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:zolana_mobile/zolana_mobile.dart';

const _both = String.fromEnvironment('ZOLANA_TIMING_RUN') == 'both';
const _shapes = String.fromEnvironment(
  'ZOLANA_TIMING_SHAPES',
  defaultValue: '1x2,2x2,4x2,8x2,16x2,24x2,merge-24x1',
);
const _rounds = int.fromEnvironment('ZOLANA_TIMING_ROUNDS', defaultValue: 4);

/// Shapes over eight notes run two rounds, a first and a warm one: their
/// proofs are long, and steadier than short ones.
int _roundsOf(int inputs) => inputs > 8 && _rounds > 2 ? 2 : _rounds;

/// The pinned key a shape proves with.
String _keyOf(String shape) => shape.startsWith('merge-')
    ? 'merge_${shape.substring('merge-'.length).replaceFirst('x', '_')}.key'
    : 'transfer_confidential_${shape.replaceFirst('x', '_')}.key';

/// A fixed account: opening a wallet needs keys, proving a fixture needs no
/// account. Nothing of this account is ever sent anywhere.
const _publicKey = 'GmaDrppBC7P5ARKV8g3djiwP89vz1jLK23V2GBjuAEGB';
const _viewingPrivate =
    '1fb49d0ea7f1c6fead90b0e1eccd87a9a7d08b3cd450412f685fa4bc88540c2a';
const _viewingPublic =
    '0299cff79e0e9e5a109e998f3868a1c358eb82e30ee4737ce8271c1e3a473ea0f6';
const _nullifierPrivate =
    'f172e55a6d39bf1602b0ef9d7276882b68c84dea44867b491091056e36cea1';
const _nullifierPublic =
    '0ce02d7711322665f107415edd1ce95cc2aa12378f6cd56d6e0544b800c448d1';

Uint8List _bytes(String hex) => Uint8List.fromList([
  for (var i = 0; i < hex.length; i += 2)
    int.parse(hex.substring(i, i + 2), radix: 16),
]);

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  // The same directory in both runs: the restart run finds the keys. Not
  // under the temporary directory, which on Android is the code cache that
  // a reinstall empties: beside it, in the app's data (Android) or
  // Library/Caches (iOS).
  final appDir = Directory.systemTemp.parent.path;
  final keyDir = Platform.isIOS
      ? '$appDir/Library/Caches/zolana-proof-timing'
      : '$appDir/zolana-proof-timing';
  final keys = Directory(keyDir);
  final mode = _both
      ? 'both'
      : keys.existsSync() && keys.listSync().isNotEmpty
      ? 'restart'
      : 'fresh';
  final runs = _both ? const ['fresh', 'restart'] : [mode];

  setUpAll(initZolanaMobile);

  testWidgets('times the proofs of the fixture requests ($mode)', (
    tester,
  ) async {
    // A shape that fails prints an `error` line, which the report shows;
    // the other shapes still run, and the test fails at the end.
    final failed = <String>[];
    for (final run in runs) {
      final wallet = await ZolanaWallet.openWithKeys(
        config: WalletConfig(
          rpcUrl: 'http://127.0.0.1:1',
          indexerUrl: 'http://127.0.0.1:1',
          provingKeyDir: keyDir,
          mints: const [],
        ),
        solanaPublicKey: _publicKey,
        keys: WalletKeys(
          viewingPrivateKey: _bytes(_viewingPrivate),
          viewingPublicKey: _bytes(_viewingPublic),
          nullifierPrivateKey: _bytes(_nullifierPrivate),
          nullifierPublicKey: _bytes(_nullifierPublic),
        ),
      );
      try {
        if (run == 'fresh') await wallet.clearProvingKeys();
        for (final shape in _shapes.split(',')) {
          try {
            await _time(wallet, shape, run);
          } catch (error) {
            failed.add(shape);
            _emit({
              'run': run,
              'shape': shape,
              'error': '$error'.split('\n').first,
            });
          }
        }
      } finally {
        // Closing drops the loaded keys; a following run loads them again.
        await wallet.close();
      }
    }
    expect(failed, isEmpty, reason: 'shapes that failed');
  }, timeout: const Timeout(Duration(minutes: 90)));
}

Future<void> _time(ZolanaWallet wallet, String shape, String run) async {
  final request = await rootBundle.loadString(
    'assets/proving/prove-request-$shape.json',
  );
  final keyName = _keyOf(shape);
  final inputs = int.parse(shape.replaceFirst('merge-', '').split('x').first);
  for (var round = 0; round < _roundsOf(inputs); round++) {
    final proved = await wallet.proveRequest(
      keyName: keyName,
      requestJson: request,
    );
    expect(jsonDecode(proved.proofJson), isA<Map<String, Object?>>());
    if (run == 'restart') {
      // Else the restart rows would time a download again.
      expect(
        proved.downloadedBytes.toInt(),
        0,
        reason: '$keyName is not on the device after the fresh run',
      );
    }
    _emit({
      'run': run,
      'shape': shape,
      'round': round,
      'downloaded_bytes': proved.downloadedBytes.toInt(),
      'download_ms': proved.downloadMs.toInt(),
      'load_ms': proved.loadMs.toInt(),
      'witness_ms': proved.witnessMs.toInt(),
      'prove_ms': proved.proveMs.toInt(),
      'verify_ms': proved.verifyMs.toInt(),
      'total_ms': proved.totalMs.toInt(),
    });
  }
}

/// One line the report script reads from the test output.
void _emit(Map<String, Object?> line) {
  // ignore: avoid_print
  print('ZOLANA_TIMING ${jsonEncode(line)}');
}
