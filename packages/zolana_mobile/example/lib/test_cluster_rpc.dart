import 'dart:convert';
import 'dart:io';

/// The plain Solana calls the demo makes with its own RPC client: the public
/// balances and the devnet airdrop that funds a demo account. Everything else
/// goes through the Zolana wallet.
class TestClusterRpc {
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
