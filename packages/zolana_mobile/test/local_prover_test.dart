import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:zolana_mobile/zolana_mobile.dart';

class ControlledBackend implements ProverBackend {
  Completer<LocalProofResult> pending = Completer();
  int proofs = 0;
  int releases = 0;
  bool failRelease = false;
  bool failLoad = false;
  bool? structured;

  @override
  Future<PreparedProverInfo> load({
    required String r1csPath,
    required String provingKeyPath,
    required String verifyingKeyPath,
  }) async {
    if (failLoad) throw StateError('private-sentinel');
    return PreparedProverInfo(id: BigInt.one, loadMs: BigInt.from(12));
  }

  @override
  Future<LocalProofResult> prove({
    required BigInt id,
    required String inputJson,
    required bool structuredRequest,
  }) {
    proofs++;
    structured = structuredRequest;
    return pending.future;
  }

  @override
  Future<void> release(BigInt id) async {
    releases++;
    if (failRelease) throw StateError('private-sentinel');
  }
}

LocalProofResult proofResult() => LocalProofResult(
  proofJson: '{}',
  verified: true,
  inputs: 2,
  outputs: 3,
  proofMs: BigInt.one,
  witnessMs: BigInt.zero,
  verifyMs: BigInt.one,
  totalMs: BigInt.two,
);

Matcher proverError(ProverErrorCode code) =>
    isA<ProverException>().having((error) => error.code, 'code', code);

Future<LocalProver> open(ControlledBackend backend) => LocalProver.load(
  r1csPath: 'circuit',
  provingKeyPath: 'pk',
  verifyingKeyPath: 'vk',
  backend: backend,
);

void main() {
  test('load failures never expose backend text', () async {
    await expectLater(
      open(ControlledBackend()..failLoad = true),
      throwsA(proverError(ProverErrorCode.loadFailed)),
    );
  });

  test('concurrent close calls share one drain and one release', () async {
    final backend = ControlledBackend();
    final prover = await open(backend);
    final job = prover.proveWitness('{}');
    final firstClose = prover.close();
    final secondClose = prover.close();
    expect(identical(firstClose, secondClose), isTrue);
    backend.pending.complete(proofResult());
    await Future.wait([firstClose, secondClose, job.done]);
    expect(backend.releases, 1);
  });
  test('rejects overlapping jobs and reuses the loaded prover', () async {
    final backend = ControlledBackend();
    final prover = await open(backend);
    final first = prover.proveWitness('{}');
    expect(
      () => prover.proveWitness('{}'),
      throwsA(proverError(ProverErrorCode.busy)),
    );
    expect(backend.proofs, 1);
    backend.pending.complete(proofResult());
    expect((await first.result).verified, isTrue);
    await first.done;
    backend.pending = Completer();
    final second = prover.proveRequest('{}');
    expect(backend.structured, isTrue);
    expect(second.id, greaterThan(first.id));
    backend.pending.complete(proofResult());
    await second.result;
    await prover.close();
    expect(backend.releases, 1);
  });

  test(
    'lock rejects new jobs immediately but drains native work before release',
    () async {
      final backend = ControlledBackend();
      final prover = await open(backend);
      final job = prover.proveWitness('{}');
      final rejected = expectLater(
        job.result,
        throwsA(proverError(ProverErrorCode.discarded)),
      );
      final closing = prover.close();
      expect(prover.isClosed, isTrue);
      expect(job.isDiscarded, isTrue);
      expect(backend.releases, 0);
      expect(
        () => prover.proveWitness('{}'),
        throwsA(proverError(ProverErrorCode.closed)),
      );
      backend.pending.complete(proofResult());
      await rejected;
      await closing;
      await prover.close();
      expect(backend.releases, 1);
    },
  );

  test('discard does not pretend the native job has stopped', () async {
    final backend = ControlledBackend();
    final prover = await open(backend);
    final job = prover.proveWitness('{}');
    job.discard();
    var finished = false;
    unawaited(job.done.then((_) => finished = true));
    await Future<void>.delayed(Duration.zero);
    expect(finished, isFalse);
    expect(
      () => prover.proveWitness('{}'),
      throwsA(proverError(ProverErrorCode.busy)),
    );
    backend.pending.completeError(StateError('private-sentinel'));
    await expectLater(
      job.result,
      throwsA(proverError(ProverErrorCode.discarded)),
    );
    await job.done;
    expect(finished, isTrue);
    await prover.close();
  });

  test('native errors are sanitized and completion always settles', () async {
    final backend = ControlledBackend();
    final prover = await open(backend);
    final job = prover.proveWitness('{}');
    backend.pending.completeError(StateError('private-sentinel'));
    await expectLater(
      job.result,
      throwsA(proverError(ProverErrorCode.proveFailed)),
    );
    await job.done;
    await prover.close();
    expect(backend.releases, 1);
  });

  test('failed release can be retried without reopening the session', () async {
    final backend = ControlledBackend()..failRelease = true;
    final prover = await open(backend);
    await expectLater(
      prover.close(),
      throwsA(proverError(ProverErrorCode.releaseFailed)),
    );
    backend.failRelease = false;
    await prover.close();
    expect(prover.isClosed, isTrue);
    expect(backend.releases, 2);
  });
}
