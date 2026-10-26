import 'dart:typed_data';

import 'package:flutter/foundation.dart' show listEquals;
import 'package:zolana_mobile/zolana_mobile.dart';

import 'encoding.dart';

const shieldedPoolProgram = 'spp3CfN2Y7se74A27bBpnAkmjXyEb2YmmZafhFrC9xt';
const registryProgram = 'regy3WAeRXMYuMYxr5iK3Bf9Arr3PcxGxyzAddbiqDQ';
const _systemProgram = '11111111111111111111111111111111';
const _associatedTokenProgram = 'ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL';
const _tokenPrograms = {
  'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA',
  'TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb',
};

/// Independent account reads from the app's configured, trusted Solana RPC.
abstract interface class SpendCheckAccounts {
  Future<Uint8List> registeredNullifier(String owner);
  Future<int> outputTreeId(String address);
  Future<bool> tokenAccountMatches(
    String account,
    String owner,
    String mint,
    String tokenProgram,
  );
}

typedef PoseidonHash = Future<Uint8List> Function(List<Uint8List> inputs);

/// The UI's payment request, captured before the SDK prepares a transaction.
class SpendRequest {
  SpendRequest({required List<Output> outputs, this.mint})
    : outputs = List.unmodifiable(outputs);

  final List<Output> outputs;
  final String? mint;
}

/// App policy for the pinned SDK's default-ring spends. Unknown formats,
/// circuits and accompanying instructions fail closed. This checks committed
/// payment facts, not ciphertext decryptability or proof validity; SPP verifies
/// the proof when executing. Do not log openings or account keys.
class SpendChecks {
  SpendChecks(this.accounts, {PoseidonHash? hash})
    : _hash = hash ?? ((inputs) => poseidonHash(inputs: inputs));

  final SpendCheckAccounts accounts;
  final PoseidonHash _hash;

  Future<List<String>> verify({
    required Uint8List message,
    required List<OutputOpening> openings,
    required String sender,
    required SpendRequest request,
  }) async {
    _require(request.outputs.isNotEmpty, 'Missing payment request');
    final parsed = _Message.read(message);
    _require(
      parsed.signers.length == 1 && parsed.signers.single == sender,
      'Unexpected signer or fee payer',
    );
    final spends = parsed.instructions.where(
      (ix) => ix.program == shieldedPoolProgram,
    );
    _require(spends.length == 1, 'Expected one shielded spend');
    final spend = spends.single;
    _require(
      spend.accounts.length >= 4 && spend.accounts[0] == sender,
      'Unexpected spend payer',
    );
    final data = _Transact.read(spend.data, spend.accounts);
    _require(openings.length == data.outputSlots, 'Missing output openings');
    final treeId = await accounts.outputTreeId(spend.accounts[1]);

    final nullifiers = <String, Uint8List>{};
    Future<Uint8List> nullifier(String owner) async =>
        nullifiers[owner] ??= await accounts.registeredNullifier(owner);
    final expected = <String, BigInt>{};
    final actual = <String, BigInt>{};
    final public = <Output_Public>[];
    for (final output in request.outputs) {
      switch (output) {
        case Output_Private(:final recipient, :final amount):
          _require(amount > BigInt.zero, 'Invalid requested amount');
          await nullifier(recipient);
          expected[recipient] = (expected[recipient] ?? BigInt.zero) + amount;
        case Output_Public():
          _require(output.amount > BigInt.zero, 'Invalid requested amount');
          public.add(output);
      }
    }
    await nullifier(sender);
    var published = 0;
    var padding = false;
    var compact = false;
    for (final opening in openings) {
      _require(opening.treeId == treeId, 'Output tree mismatch');
      final owner = opening.ownerSigningKey;
      if (owner == null) {
        padding = true;
        _require(
          opening.nullifierKey == null && opening.amount == BigInt.zero,
          'Invalid dummy output',
        );
      } else {
        _require(!padding && !opening.compact, 'Invalid output order');
        // The SDK's signing key encoding is scheme || 32-byte key || zero.
        _require(
          owner.length == 34 && owner[0] == 1 && owner[33] == 0,
          'Unsupported output owner',
        );
        final recipient = base58Encode(owner.sublist(1, 33));
        final registered = nullifiers[recipient];
        _require(
          registered != null && listEquals(registered, opening.nullifierKey),
          'Unapproved output owner',
        );
        _require(
          opening.mint == request.mint && opening.amount > BigInt.zero,
          'Unexpected output asset or amount',
        );
        _require(
          opening.dataHash == null &&
              opening.ringDataHash == null &&
              opening.ringProgramId == null,
          'Unexpected output data or ring',
        );
        actual[recipient] = (actual[recipient] ?? BigInt.zero) + opening.amount;
      }
      if (opening.compact) {
        compact = true;
        _require(owner == null, 'Compact real output');
      } else {
        _require(
          !compact && published < data.commitments.length,
          'Unexpected output slot',
        );
        if (owner != null) {
          _require(
            listEquals(data.ownerTags[published], owner.sublist(1, 33)),
            'Output discovery tag mismatch',
          );
        }
        final commitment = await _commitment(opening);
        _require(
          listEquals(commitment, data.commitments[published++]),
          'Output commitment mismatch',
        );
      }
    }
    _require(
      published == data.commitments.length,
      'Unopened output commitment',
    );
    for (final entry in expected.entries) {
      final received = actual.remove(entry.key) ?? BigInt.zero;
      // A self-payment and change share one owner. All excess stays with sender.
      _require(
        entry.key == sender ? received >= entry.value : received == entry.value,
        'Private payment amount mismatch',
      );
    }
    actual.remove(sender);
    _require(actual.isEmpty, 'Unexpected private payment');

    final created = <String, _CreatedTokenAccount>{};
    for (final ix in parsed.instructions) {
      if (identical(ix, spend)) continue;
      _require(
        ix.program == _associatedTokenProgram &&
            listEquals(ix.data, [1]) &&
            ix.accounts.length == 6,
        'Unexpected accompanying instruction',
      );
      final keys = ix.accounts;
      _require(
        keys[0] == sender &&
            keys[4] == _systemProgram &&
            _tokenPrograms.contains(keys[5]) &&
            !created.containsKey(keys[1]),
        'Unexpected token account creation',
      );
      created[keys[1]] = _CreatedTokenAccount(keys[2], keys[3], keys[5]);
    }
    _require(
      data.transfers.length == public.length,
      'Public payment count mismatch',
    );
    final groupSize = data.transfers.fold<int>(
      0,
      (n, leg) => n + (leg.spl ? 5 : 2),
    );
    _require(
      spend.accounts.length ==
          4 + data.inputTrees + data.inputs + 1 + groupSize,
      'Unexpected spend account layout',
    );
    var offset = spend.accounts.length - groupSize;
    final usedCreations = <String>{};
    for (var i = 0; i < public.length; i++) {
      final wanted = public[i];
      final leg = data.transfers[i];
      final group = spend.accounts.sublist(offset, offset + (leg.spl ? 5 : 2));
      offset += group.length;
      _require(
        leg.amount == wanted.amount && leg.spl == (request.mint != null),
        'Public payment amount or asset mismatch',
      );
      if (!leg.spl) {
        _require(group[1] == wanted.recipient, 'Public recipient mismatch');
      } else {
        final mint = request.mint!;
        _require(
          group[1] == mint && _tokenPrograms.contains(group[4]),
          'Public mint or token program mismatch',
        );
        final setup = created[group[3]];
        if (setup != null) {
          _require(
            setup.owner == wanted.recipient &&
                setup.mint == mint &&
                setup.program == group[4],
            'Token account recipient mismatch',
          );
          usedCreations.add(group[3]);
        } else {
          _require(
            await accounts.tokenAccountMatches(
              group[3],
              wanted.recipient,
              mint,
              group[4],
            ),
            'Token account recipient mismatch',
          );
        }
      }
    }
    _require(
      usedCreations.length == created.length,
      'Unrequested token account creation',
    );
    return created.values.map((account) => account.owner).toList();
  }

  Future<Uint8List> _commitment(OutputOpening opening) async {
    final zero = Uint8List(32);
    final dummy = opening.ownerSigningKey == null;
    _require(opening.blinding.length == 32, 'Invalid output blinding');
    final owner = dummy
        ? zero
        : await _hash([
            await _bytesHash([
              0x53,
              ...opening.ownerSigningKey!.sublist(1, 33),
            ]),
            opening.nullifierKey!,
          ]);
    return _hash([
      _field(BigInt.from(dummy ? 1 : 3)),
      _field(BigInt.from(opening.treeId)),
      dummy
          ? zero
          : await _bytesHash(
              opening.mint == null ? zero : base58Decode(opening.mint!),
            ),
      dummy ? zero : _field(opening.amount),
      zero,
      await _hash([zero, zero]),
      await _hash([owner, opening.blinding]),
    ]);
  }

  Future<Uint8List> _bytesHash(List<int> bytes) => _hash([
    Uint8List.fromList([0, ...bytes.sublist(0, 31)]),
    Uint8List.fromList([
      ...List.filled(32 - (bytes.length - 31), 0),
      ...bytes.sublist(31),
    ]),
  ]);
}

class _CreatedTokenAccount {
  _CreatedTokenAccount(this.owner, this.mint, this.program);
  final String owner, mint, program;
}

class _Instruction {
  _Instruction(this.program, this.accounts, this.data);
  final String program;
  final List<String> accounts;
  final Uint8List data;
}

class _Message {
  _Message(this.signers, this.instructions);
  final List<String> signers;
  final List<_Instruction> instructions;

  static _Message read(Uint8List bytes) {
    _require(bytes.length <= 4096, 'Message exceeds v1 size limit');
    final r = _Reader(bytes);
    _require(r.u8() == 0x81, 'Expected a Solana v1 message');
    final signatures = r.u8();
    final readonlySigned = r.u8();
    final readonlyUnsigned = r.u8();
    final mask = r.uint(4).toInt();
    _require(
      mask & ~31 == 0 && (mask & 3 == 0 || mask & 3 == 3),
      'Unknown v1 configuration',
    );
    r.take(32); // Recent blockhash.
    final instructionCount = r.u8();
    final keyCount = r.u8();
    _require(
      keyCount <= 64 &&
          signatures > 0 &&
          signatures <= keyCount &&
          readonlySigned < signatures &&
          readonlyUnsigned <= keyCount - signatures &&
          instructionCount > 0 &&
          instructionCount <= 64,
      'Invalid message header',
    );
    final keys = [for (var i = 0; i < keyCount; i++) base58Encode(r.take(32))];
    _require(keys.toSet().length == keys.length, 'Duplicate message account');
    if (mask & 3 == 3) {
      _require(
        r.uint(8) <= BigInt.from(10000),
        'Priority fee exceeds demo limit',
      );
    }
    if (mask & 4 != 0) r.take(4);
    if (mask & 8 != 0) r.take(4);
    if (mask & 16 != 0) r.take(4);
    final headers = [
      for (var i = 0; i < instructionCount; i++)
        (r.u8(), r.u8(), r.uint(2).toInt()),
    ];
    final instructions = <_Instruction>[];
    for (final (program, count, length) in headers) {
      _require(program < keyCount, 'Invalid program index');
      final indexes = r.take(count);
      _require(indexes.every((i) => i < keyCount), 'Invalid account index');
      instructions.add(
        _Instruction(keys[program], [
          for (final i in indexes) keys[i],
        ], r.take(length)),
      );
    }
    r.end();
    return _Message(keys.take(signatures).toList(), instructions);
  }
}

class _Transfer {
  _Transfer(this.spl, this.amount);
  final bool spl;
  final BigInt amount;
}

class _Transact {
  _Transact(
    this.commitments,
    this.ownerTags,
    this.transfers,
    this.outputSlots,
    this.inputs,
    this.inputTrees,
  );
  final List<Uint8List> commitments;
  final List<Uint8List> ownerTags;
  final List<_Transfer> transfers;
  final int outputSlots, inputs, inputTrees;

  static _Transact read(Uint8List bytes, List<String> accounts) {
    final r = _Reader(bytes);
    _require(r.u8() == 16, 'Expected a default-ring transact');
    r.take(8 + 33 + 16); // Expiry, viewing public key and salt.
    final transfers = <_Transfer>[];
    final legs = r.u8();
    for (var i = 0; i < legs; i++) {
      final kind = r.u8();
      _require(kind == 1 || kind == 3, 'Unexpected deposit or transfer type');
      transfers.add(_Transfer(kind == 3, r.uint(8)));
      if (kind == 3) r.take(1); // SPL interface bump.
    }
    _require(r.u8() == 0 && r.u8() == 0, 'Unexpected transaction data or ring');
    final commitments = <Uint8List>[];
    final ownerTags = <Uint8List>[];
    final outputs = r.u8();
    for (var i = 0; i < outputs; i++) {
      commitments.add(r.take(32));
      switch (r.u8()) {
        case 0:
          ownerTags.add(r.take(32));
        case 1:
          final index = r.u8();
          _require(index < accounts.length, 'Invalid owner tag account');
          ownerTags.add(base58Decode(accounts[index]));
        default:
          throw StateError('Unknown output owner tag');
      }
      final hasData = r.u8();
      _require(hasData <= 1, 'Invalid ciphertext option');
      if (hasData == 1) r.take(r.uint(2).toInt());
    }
    _require(r.u8() == 0, 'Unexpected transaction messages');
    r.take(32); // Private transaction hash; its preimage stays inside the SDK.
    _require(r.uint(2) == BigInt.zero, 'Unsupported spend circuit or cache');
    final inputs = r.u8();
    final outputSlots = r.u8();
    _require(
      r.u8() == 3 &&
          outputSlots >= outputs &&
          outputSlots <= 16 &&
          inputs > 0 &&
          inputs <= 40,
      'Unsupported spend shape',
    );
    r.take(192); // Compressed Groth16 proof, verified by SPP.
    final publishedInputs = r.u8();
    _require(publishedInputs <= inputs, 'Unexpected input count');
    r.take(publishedInputs * 33);
    final inputTrees = r.u8();
    _require(inputTrees > 0 && inputTrees <= 5, 'Unexpected input trees');
    r.take(inputTrees * 4);
    r.end();
    return _Transact(
      commitments,
      ownerTags,
      transfers,
      outputSlots,
      publishedInputs,
      inputTrees,
    );
  }
}

class _Reader {
  _Reader(this.bytes);
  final Uint8List bytes;
  int offset = 0;
  Uint8List take(int count) {
    _require(
      count >= 0 && count <= bytes.length - offset,
      'Truncated transaction',
    );
    final result = Uint8List.fromList(bytes.sublist(offset, offset + count));
    offset += count;
    return result;
  }

  int u8() => take(1).single;
  BigInt uint(int count) {
    var value = BigInt.zero;
    for (final byte in take(count).reversed) {
      value = (value << 8) | BigInt.from(byte);
    }
    return value;
  }

  void end() => _require(offset == bytes.length, 'Trailing transaction data');
}

Uint8List _field(BigInt value) {
  _require(
    value >= BigInt.zero && value.bitLength <= 256,
    'Invalid field value',
  );
  final bytes = Uint8List(32);
  for (var i = 31; i >= 0; i--) {
    bytes[i] = (value & BigInt.from(255)).toInt();
    value >>= 8;
  }
  return bytes;
}

void _require(bool condition, String reason) {
  if (!condition) throw StateError('Spend check failed: $reason');
}
