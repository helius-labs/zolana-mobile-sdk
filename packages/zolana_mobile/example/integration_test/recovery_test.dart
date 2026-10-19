import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:path_provider/path_provider.dart';
import 'package:zolana_mobile/zolana_mobile.dart';
import 'package:zolana_mobile_demo/app_transport.dart';
import 'package:zolana_mobile_demo/demo_assets.dart';
import 'package:zolana_mobile_demo/demo_keys.dart';
import 'package:zolana_mobile_demo/demo_signer.dart';
import 'package:zolana_mobile_demo/recovery.dart';
import 'package:zolana_mobile_demo/wallet_screen.dart' show Network;

/// The recovery cases of `lib/recovery.dart` on Zolana devnet, with demo
/// account B: restoring on a new phone from the user's signature, reopening
/// from saved keys without one, and a send found again after a restart or
/// sent again after it expired. B pays itself 0.0001 SOL twice, so its
/// private balance does not move; it pays the network fees in public SOL.
/// Opt in, since it spends devnet SOL and needs a Helius key for devnet:
///
/// ```sh
/// flutter test integration_test/recovery_test.dart -d DEVICE \
///   --dart-define=ZOLANA_E2E=true --dart-define=ZOLANA_API_KEY=...
/// ```
const _enabled = bool.fromEnvironment('ZOLANA_E2E');
const _apiKey = String.fromEnvironment('ZOLANA_API_KEY');

const _derivation = 'Open your private Zolana wallet';
final _account = demoAccounts[1];
final _amount = BigInt.from(100000);

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  setUpAll(initZolanaMobile);

  Future<WalletConfig> config({String? provingKeyDir}) async {
    expect(
      _apiKey,
      isNotEmpty,
      reason: 'pass --dart-define=ZOLANA_API_KEY=...',
    );
    return WalletConfig(
      rpcUrl: Network.devnet.rpcUrl,
      indexerUrl: Network.devnet.indexerUrl,
      provingKeyDir:
          provingKeyDir ??
          '${(await getApplicationSupportDirectory()).path}/proving-keys',
      mints: [for (final asset in demoAssets) ?asset.config],
    );
  }

  /// B's signer, which adds what it signs to [approvals].
  Future<DemoSigner> signer([List<String>? approvals]) =>
      DemoSigner.fromSeedHex(_account.seedHex, onSign: approvals?.add);

  Future<PreparedTransaction> payItself(ZolanaWallet wallet) =>
      wallet.prepareSpend(
        outputs: [
          Output.private(recipient: _account.publicKey, amount: _amount),
        ],
      );

  testWidgets('restores on a new phone from the signature', (tester) async {
    final approvals = <String>[];
    final old = await ZolanaWallet.open(
      signer: await signer(),
      config: await config(),
    );
    addTearDown(old.close);
    // The new phone holds no keys, notes or proving keys.
    final empty = await Directory.systemTemp.createTemp('new-phone-');
    addTearDown(() => empty.delete(recursive: true));
    final phone = await openWallet(
      config: await config(provingKeyDir: empty.path),
      signer: await signer(approvals),
      vault: MemoryKeyVault(),
    );
    addTearDown(phone.close);

    expect(approvals, [_derivation]);
    expect(phone.shieldedAddress, old.shieldedAddress);
    expect(await phone.registrationStatus(), RegistrationStatus.registered);
    final [restored, original] = await Future.wait([
      phone.overview(),
      old.overview(),
    ]);
    expect(restored.balances, original.balances);
    expect(
      restored.activity.map((entry) => entry.signature),
      original.activity.map((entry) => entry.signature),
    );
  }, skip: !_enabled);

  testWidgets('reopens from saved keys without a signature', (tester) async {
    final approvals = <String>[];
    final vault = MemoryKeyVault();
    final first = await openWallet(
      config: await config(),
      signer: await signer(approvals),
      vault: vault,
    );
    await first.close();
    expect(approvals, [_derivation]);

    final reopened = await openWallet(
      config: await config(),
      signer: await signer(approvals),
      vault: vault,
    );
    addTearDown(reopened.close);
    expect(approvals, [_derivation]);
    expect(reopened.shieldedAddress, first.shieldedAddress);
    expect(await reopened.registrationStatus(), RegistrationStatus.registered);
    await reopened.close();

    // Keys the keystore damaged are derived again from the signature.
    final saved = (await vault.read(_account.publicKey))!;
    await vault.write(
      _account.publicKey,
      WalletKeys(
        viewingPrivateKey: Uint8List(32),
        viewingPublicKey: saved.viewingPublicKey,
        nullifierPrivateKey: saved.nullifierPrivateKey,
        nullifierPublicKey: saved.nullifierPublicKey,
      ),
    );
    final repaired = await openWallet(
      config: await config(),
      signer: await signer(approvals),
      vault: vault,
    );
    addTearDown(repaired.close);
    expect(approvals, [_derivation, _derivation]);
    expect(repaired.shieldedAddress, first.shieldedAddress);
    expect(
      (await vault.read(_account.publicKey))!.viewingPrivateKey,
      saved.viewingPrivateKey,
    );
  }, skip: !_enabled);

  testWidgets('finds a send again after a restart', (tester) async {
    final vault = MemoryKeyVault();
    final pending = MemoryPendingSends();
    final network = _Network();
    addTearDown(network.close);
    final wallet = await openWallet(
      config: await config(),
      signer: await signer(),
      vault: vault,
      transport: network.send,
    );
    addTearDown(wallet.close);
    expect(await settlePendingSend(wallet, pending), isNull);
    expect(await wallet.privateBalance(), greaterThanOrEqualTo(_amount));
    final transaction = await payItself(wallet);

    // The network is gone once the transaction is out, as when the app is
    // closed while sending: whether it landed is not known.
    network.offlineAfterSend = true;
    await expectLater(
      saveAndSend(wallet, await signer(), pending, transaction),
      throwsA(isA<WalletError_TransactionOutcomeUnknown>()),
    );
    final signature = (await pending.read(_account.publicKey))?.signature;
    expect(signature, isNotNull);
    await wallet.close();

    // The next launch waits for it before anything else is sent.
    final restarted = await openWallet(
      config: await config(),
      signer: await signer(),
      vault: vault,
    );
    addTearDown(restarted.close);
    expect(await settlePendingSend(restarted, pending), signature);
    expect(await pending.read(_account.publicKey), isNull);
    expect(
      (await restarted.activity()).map((entry) => entry.signature),
      contains(signature),
    );
  }, skip: !_enabled);

  testWidgets('sends again after a send expired', (tester) async {
    final pending = MemoryPendingSends();
    final network = _Network();
    addTearDown(network.close);
    final wallet = await openWallet(
      config: await config(),
      signer: await signer(),
      vault: MemoryKeyVault(),
      transport: network.send,
    );
    addTearDown(wallet.close);
    final transaction = await payItself(wallet);

    // The send never reaches the network, and the blockhash expires.
    network.dropSends = true;
    String? expired;
    try {
      await saveAndSend(wallet, await signer(), pending, transaction);
    } on WalletError_TransactionExpired catch (error) {
      expired = error.signature;
    }
    expect(expired, isNotNull);
    expect(await pending.read(_account.publicKey), isNull);

    // It can no longer land: the same proof under a new blockhash is safe
    // to send.
    network.dropSends = false;
    final signature = await saveAndSend(
      wallet,
      await signer(),
      pending,
      await wallet.refresh(transaction),
    );
    expect(await pending.read(_account.publicKey), isNull);
    final sent = (await wallet.activity()).map((entry) => entry.signature);
    expect(sent, contains(signature));
    expect(sent, isNot(contains(expired)));
  }, skip: !_enabled);

  testWidgets('drops a send that never went out once it expired', (
    tester,
  ) async {
    final pending = MemoryPendingSends();
    final vault = MemoryKeyVault();
    final wallet = await openWallet(
      config: await config(),
      signer: await signer(),
      vault: vault,
    );
    final transaction = await payItself(wallet);
    // The application saved the send and was closed before it went out.
    final signed = await (await signer()).signMessage(
      transaction.message,
      transaction: transaction,
    );
    await pending.write(_account.publicKey, (
      signature: base58Encode(signed),
      lastValidBlockHeight: transaction.lastValidBlockHeight,
    ));
    await wallet.close();

    // The next launch finds it never landed: sending again is safe.
    final restarted = await openWallet(
      config: await config(),
      signer: await signer(),
      vault: vault,
    );
    addTearDown(restarted.close);
    await expectLater(
      settlePendingSend(restarted, pending),
      throwsA(isA<WalletError_TransactionExpired>()),
    );
    expect(await pending.read(_account.publicKey), isNull);
  }, skip: !_enabled);
}

/// The application's transport on a network that fails around a send.
class _Network {
  final _transport = AppTransport();

  /// Each `sendTransaction` fails before it goes out.
  bool dropSends = false;

  /// The next `sendTransaction` goes out, and every request after it fails.
  bool offlineAfterSend = false;

  bool _offline = false;

  Future<TransportResponse> send(TransportRequest request) async {
    final sending = utf8
        .decode(request.body, allowMalformed: true)
        .contains('"sendTransaction"');
    if (_offline || (sending && dropSends)) {
      throw const SocketException('Network is unreachable');
    }
    final response = await _transport.send(request);
    if (sending && offlineAfterSend) {
      _offline = true;
      throw const SocketException('Network is unreachable');
    }
    return response;
  }

  void close() => _transport.close();
}
