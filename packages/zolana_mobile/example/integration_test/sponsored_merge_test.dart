import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:path_provider/path_provider.dart';
import 'package:zolana_mobile/zolana_mobile.dart';
import 'package:zolana_mobile_demo/demo_assets.dart';
import 'package:zolana_mobile_demo/demo_keys.dart';
import 'package:zolana_mobile_demo/demo_signer.dart';
import 'package:zolana_mobile_demo/prover_policy.dart';
import 'package:zolana_mobile_demo/sponsored_merge.dart';
import 'package:zolana_mobile_demo/spend_checks.dart';
import 'package:zolana_mobile_demo/test_cluster_rpc.dart';
import 'package:zolana_mobile_demo/wallet_screen.dart' show Network;

/// Opt-in live test against the example backend. Uses the public devnet demo
/// accounts and spends devnet SOL and TEST tokens; see backend/README.md.
const _enabled = bool.fromEnvironment('ZOLANA_E2E');
const _backendUrl = String.fromEnvironment(
  'ZOLANA_BACKEND_URL',
  defaultValue: 'http://127.0.0.1:8788/',
);
const _auth = String.fromEnvironment('ZOLANA_BACKEND_AUTH');
const _requireQueued = bool.fromEnvironment('ZOLANA_E2E_REQUIRE_QUEUED');

// Stop after the first relayer response was persisted, before cache confirmation.
// Reopening must restore reservations and replay the same merge before round two.
class _StopAfterFirstMerge extends SponsoredStore {
  _StopAfterFirstMerge(SponsoredStore store) : super(store.file, store.key);

  @override
  Future<void> write(Map<String, dynamic> state) async {
    await super.write(state);
    final merges = state['merges'] as List;
    if (merges.length == 1 && merges.first['signature'] != null) {
      throw StateError('Simulated restart after first merge');
    }
  }
}

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(initZolanaMobile);

  for (final asset in demoAssets) {
    testWidgets(
      'sponsored ${asset.symbol}: TEE merges, recovery and local payment',
      (tester) async {
        expect(_auth, isNotEmpty);
        final backend = SponsoredBackend(Uri.parse(_backendUrl), _auth);
        addTearDown(backend.close);
        final directory = await (await getTemporaryDirectory()).createTemp(
          'sponsored-e2e-',
        );
        addTearDown(() => directory.delete(recursive: true));
        var proofPosts = 0;
        var statusReads = 0;
        Future<TransportResponse> transport(TransportRequest request) {
          final uri = Uri.parse(request.url);
          if (request.url.startsWith('${backend.proverUrl}/prove/')) {
            expect(uri.path, contains('/merge_'));
            if (request.method == 'POST') {
              proofPosts++;
              // Request queued delivery; deployments without a queue may
              // return the proof inline. Headers are outside the TEE payload.
              final headers = {...request.headers}
                ..remove('x-sync')
                ..['x-async'] = 'true';
              request = TransportRequest(
                method: request.method,
                url: request.url,
                headers: headers,
                body: request.body,
                timeoutMs: request.timeoutMs,
                maxResponseBytes: request.maxResponseBytes,
                downloadPath: request.downloadPath,
              );
            } else {
              expect(request.method, 'GET');
              expect(uri.path, endsWith('/status'));
              statusReads++;
            }
          }
          return backend.transport(request);
        }

        final signer = await DemoSigner.fromSeedHex(demoAccounts[0].seedHex);
        final config = WalletConfig(
          rpcUrl: Network.devnet.rpcUrl,
          indexerUrl: Network.devnet.indexerUrl,
          provingKeyDir:
              '${(await getApplicationSupportDirectory()).path}/proving-keys',
          proverUrl: backend.proverUrl,
          requireProverTee: true,
          proverTeePolicy: await demoProverPolicy(),
          mints: [for (final asset in demoAssets) ?asset.config],
        );
        Future<ZolanaWallet> open() => ZolanaWallet.open(
          signer: signer,
          config: config,
          transport: transport,
        );
        var wallet = await open();
        addTearDown(() => wallet.close());
        final receiver = await ZolanaWallet.open(
          signer: await DemoSigner.fromSeedHex(demoAccounts[1].seedHex),
          config: config,
          transport: transport,
        );
        addTearDown(receiver.close);
        await wallet.register();
        await receiver.register();
        await wallet.setMerging(true);
        final receiverBefore = await receiver.privateBalance(mint: asset.mint);
        for (var i = 0; i < 4; i++) {
          await wallet.deposit(BigInt.from(1000000), mint: asset.mint);
        }

        var store = await SponsoredStore.forWallet(directory, wallet);
        final checks = SpendChecks(TestClusterRpc(Network.devnet.rpcUrl));
        SponsoredFlow flow(SponsoredStore store) =>
            SponsoredFlow(wallet, signer, backend, store, checks);
        var run = flow(_StopAfterFirstMerge(store));
        await run.start(rounds: 2, treeId: 0, mint: asset.mint, maxInputs: 2);
        await expectLater(
          run.mergeRounds(),
          throwsA(
            isA<StateError>().having(
              (error) => error.message,
              'message',
              'Simulated restart after first merge',
            ),
          ),
        );
        final firstSignature =
            (run.state!['merges'] as List).first['signature'];
        expect(firstSignature, isNotNull);
        await wallet.close();
        wallet = await open();
        store = await SponsoredStore.forWallet(directory, wallet);
        run = flow(store);
        await run.restore();
        await run.mergeRounds();
        expect(run.phase, 'ready');
        expect(
          (run.state!['merges'] as List).first['signature'],
          firstSignature,
        );
        expect(
          proofPosts,
          2,
          reason: 'restoring must not reprove the first merge',
        );
        if (_requireQueued) expect(statusReads, greaterThanOrEqualTo(2));

        var total = BigInt.zero;
        for (final merge in run.state!['merges'] as List) {
          final pending = await wallet.restorePreparedTransaction(
            merge['snapshot'] as String,
          );
          expect(pending.signers, [pending.feePayer]);
          expect(pending.feePayer, isNot(signer.publicKey));
          expect(pending.priorityFeeLamports, BigInt.from(5000));
          total += pending.cachedOutput!.amount;
        }
        final amount = total ~/ BigInt.from(4);
        final outputs = [
          Output.private(recipient: receiver.solanaPublicKey, amount: amount),
          Output.public(recipient: signer.publicKey, amount: amount),
        ];
        await run.preparePayment(outputs);
        final message = run.payment!.message;
        await wallet.close();
        wallet = await open();
        store = await SponsoredStore.forWallet(directory, wallet);
        run = flow(store);
        await run.restore();
        expect(run.phase, 'approval');
        expect(run.payment!.message, orderedEquals(message));
        await run.refreshPayment();
        var approvals = 0;
        final signature = await run.signAndSend((payment) async {
          approvals++;
          expect(payment.signers, [signer.publicKey]);
          expect(payment.priorityFeeLamports, BigInt.from(5000));
          return true;
        });
        expect(approvals, 1);
        expect(proofPosts, 2, reason: 'final payment must be proved locally');
        expect(run.complete, isTrue);
        expect((await store.read())!['phase'], 'complete');
        expect(
          await receiver.privateBalance(mint: asset.mint),
          receiverBefore + amount,
        );
        await wallet.close();
        wallet = await open();
        final lookup = await wallet.transaction(signature);
        expect(lookup.status, TransactionLookupStatus.ready);
        expect(lookup.entries, isNotEmpty);
        // ignore: avoid_print
        print(
          '${asset.symbol}: $signature; $proofPosts TEE merge proofs, '
          '$statusReads status requests, two wallet restarts, local final proof',
        );
      },
      skip: !_enabled,
      timeout: const Timeout(Duration(minutes: 10)),
    );
  }
}
