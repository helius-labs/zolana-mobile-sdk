import 'dart:async';

import 'rust/third_party/zolana_mobile.dart';

enum ProverErrorCode {
  busy,
  closed,
  discarded,
  loadFailed,
  proveFailed,
  releaseFailed,
}

class ProverException implements Exception {
  const ProverException(this.code);

  final ProverErrorCode code;

  @override
  String toString() => 'Zolana prover: ${code.name}';
}

abstract interface class ProverBackend {
  Future<PreparedProverInfo> load({
    required String r1csPath,
    required String provingKeyPath,
    required String verifyingKeyPath,
  });
  Future<LocalProofResult> prove({
    required BigInt id,
    required String inputJson,
    required bool structuredRequest,
  });
  Future<void> release(BigInt id);
}

class NativeProverBackend implements ProverBackend {
  const NativeProverBackend();

  @override
  Future<PreparedProverInfo> load({
    required String r1csPath,
    required String provingKeyPath,
    required String verifyingKeyPath,
  }) => loadProver(
    r1CsPath: r1csPath,
    provingKeyPath: provingKeyPath,
    verifyingKeyPath: verifyingKeyPath,
  );

  @override
  Future<LocalProofResult> prove({
    required BigInt id,
    required String inputJson,
    required bool structuredRequest,
  }) => provePrepared(
    id: id,
    inputJson: inputJson,
    structuredRequest: structuredRequest,
  );

  @override
  Future<void> release(BigInt id) => releaseProver(id: id);
}

class ProofJob {
  ProofJob._(this.id);

  final int id;
  final Completer<LocalProofResult> _result = Completer();
  final Completer<void> _done = Completer();
  bool _discarded = false;

  Future<LocalProofResult> get result => _result.future;
  Future<void> get done => _done.future;
  bool get isDiscarded => _discarded;

  void discard() {
    if (!_done.isCompleted) _discarded = true;
  }

  Future<void> _run(
    Future<LocalProofResult> Function() operation,
    void Function() settled,
  ) async {
    _result.future.ignore();
    try {
      final proof = await operation();
      if (_discarded) {
        _result.completeError(const ProverException(ProverErrorCode.discarded));
      } else {
        _result.complete(proof);
      }
    } catch (_) {
      _result.completeError(
        ProverException(
          _discarded ? ProverErrorCode.discarded : ProverErrorCode.proveFailed,
        ),
      );
    } finally {
      settled();
      _done.complete();
    }
  }
}

class LocalProver {
  LocalProver._(this._backend, this._info);

  final ProverBackend _backend;
  final PreparedProverInfo _info;
  ProofJob? _active;
  Future<void>? _closing;
  bool _closed = false;
  bool _released = false;
  int _nextJobId = 1;

  BigInt get loadMs => _info.loadMs;
  bool get isClosed => _closed;

  static Future<LocalProver> load({
    required String r1csPath,
    required String provingKeyPath,
    required String verifyingKeyPath,
    ProverBackend backend = const NativeProverBackend(),
  }) async {
    try {
      final info = await backend.load(
        r1csPath: r1csPath,
        provingKeyPath: provingKeyPath,
        verifyingKeyPath: verifyingKeyPath,
      );
      return LocalProver._(backend, info);
    } catch (_) {
      throw const ProverException(ProverErrorCode.loadFailed);
    }
  }

  ProofJob proveRequest(String requestJson) => _start(requestJson, true);

  ProofJob proveWitness(String witnessJson) => _start(witnessJson, false);

  ProofJob _start(String inputJson, bool structuredRequest) {
    if (_closed) throw const ProverException(ProverErrorCode.closed);
    if (_active != null) throw const ProverException(ProverErrorCode.busy);
    final job = ProofJob._(_nextJobId++);
    _active = job;
    unawaited(
      job._run(
        () => _backend.prove(
          id: _info.id,
          inputJson: inputJson,
          structuredRequest: structuredRequest,
        ),
        () => _active = null,
      ),
    );
    return job;
  }

  Future<void> close() {
    _closed = true;
    _active?.discard();
    if (_released) return Future.value();
    return _closing ??= _drainAndRelease();
  }

  Future<void> _drainAndRelease() async {
    try {
      await _active?.done;
      await _backend.release(_info.id);
      _released = true;
    } catch (_) {
      throw const ProverException(ProverErrorCode.releaseFailed);
    } finally {
      _closing = null;
    }
  }
}
