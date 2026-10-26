import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'encoding.dart';
import 'spend_checks.dart';

/// The plain Solana calls the demo makes with its own RPC client: the public
/// balances and the devnet airdrop that funds a demo account. Everything else
/// goes through the Zolana wallet.
class TestClusterRpc implements SpendCheckAccounts {
  TestClusterRpc(this.url);

  final String url;

  /// Lamports of [publicKey], at the commitment the wallet confirms at.
  Future<BigInt> balance(String publicKey) async {
    final result = await _call('getBalance', [
      publicKey,
      {'commitment': 'confirmed'},
    ]);
    return BigInt.from((result as Map)['value'] as int);
  }

  /// Base units of [mint] in [owner]'s token accounts, at the commitment the
  /// wallet confirms at.
  Future<BigInt> tokenBalance(String owner, String mint) async {
    final result = await _call('getTokenAccountsByOwner', [
      owner,
      {'mint': mint},
      {'encoding': 'jsonParsed', 'commitment': 'confirmed'},
    ]);
    var total = BigInt.zero;
    for (final account in (result as Map)['value'] as List) {
      final amount =
          account['account']['data']['parsed']['info']['tokenAmount']['amount'];
      total += BigInt.parse(amount as String);
    }
    return total;
  }

  /// Request [lamports] and wait until the airdrop is confirmed.
  Future<void> airdrop(String publicKey, BigInt lamports) async {
    final signature =
        await _call('requestAirdrop', [publicKey, lamports.toInt()]) as String;
    for (var attempt = 0; attempt < 60; attempt++) {
      final statuses = await _call('getSignatureStatuses', [
        [signature],
      ]);
      final status = ((statuses as Map)['value'] as List).single;
      if (status != null && status['confirmationStatus'] != 'processed') {
        return;
      }
      await Future<void>.delayed(const Duration(milliseconds: 500));
    }
    throw StateError('airdrop $signature was not confirmed');
  }

  /// Read the registry program directly, independently of the wallet SDK.
  @override
  Future<Uint8List> registeredNullifier(String owner) async {
    final records = await _call('getProgramAccounts', [
      registryProgram,
      {
        'encoding': 'base64',
        'commitment': 'confirmed',
        'filters': [
          {'dataSize': 100},
          {
            'memcmp': {'offset': 1, 'bytes': owner},
          },
        ],
      },
    ]) as List;
    if (records.length != 1) throw StateError('Recipient is not registered');
    final account = records.single['account'] as Map;
    final bytes = base64Decode((account['data'] as List).first as String);
    if (account['owner'] != registryProgram ||
        bytes.length != 100 ||
        bytes[0] != 1 ||
        base58Encode(bytes.sublist(1, 33)) != owner) {
      throw StateError('Invalid recipient registration');
    }
    return Uint8List.fromList(bytes.sublist(34, 66));
  }

  @override
  Future<int> outputTreeId(String address) async {
    final result = await _call('getAccountInfo', [
      address,
      {
        'encoding': 'base64',
        'commitment': 'confirmed',
        'dataSlice': {'offset': 0, 'length': 4},
      },
    ]) as Map;
    final account = result['value'] as Map?;
    if (account == null || account['owner'] != shieldedPoolProgram) {
      throw StateError('Invalid output tree');
    }
    final bytes = base64Decode((account['data'] as List).first as String);
    if (bytes.length != 4 || bytes[0] != 1 || bytes[1] != 1) {
      throw StateError('Invalid output tree header');
    }
    return bytes[2] | bytes[3] << 8;
  }

  @override
  Future<bool> tokenAccountMatches(
    String address,
    String owner,
    String mint,
    String tokenProgram,
  ) async {
    final result = await _call('getAccountInfo', [
      address,
      {'encoding': 'jsonParsed', 'commitment': 'confirmed'},
    ]) as Map;
    final account = result['value'] as Map?;
    if (account == null || account['owner'] != tokenProgram) return false;
    final data = account['data'];
    if (data is! Map || data['parsed'] is! Map) return false;
    final parsed = data['parsed'] as Map;
    final info = parsed['info'];
    return parsed['type'] == 'account' &&
        info is Map &&
        info['owner'] == owner &&
        info['mint'] == mint;
  }

  Future<Object?> _call(String method, List<Object?> params) async {
    final client = HttpClient();
    try {
      final request = await client.postUrl(Uri.parse(url));
      request.headers.contentType = ContentType.json;
      request.write(
        jsonEncode({
          'jsonrpc': '2.0',
          'id': 1,
          'method': method,
          'params': params,
        }),
      );
      final response = await request.close();
      final body = jsonDecode(await response.transform(utf8.decoder).join());
      if (body['error'] != null) {
        throw StateError('$method failed: ${body['error']['message']}');
      }
      return body['result'];
    } on IOException {
      // Its message can name the URL, and with it the API key.
      throw StateError('$method failed: the Solana RPC is unreachable');
    } finally {
      client.close();
    }
  }
}
