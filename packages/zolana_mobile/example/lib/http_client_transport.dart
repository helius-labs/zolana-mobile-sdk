import 'dart:io';
import 'dart:typed_data';

import 'package:zolana_mobile/zolana_mobile.dart';

/// Sends the wallet's requests with `dart:io`, as an application sends them
/// through its own networking stack. Pass [send] as the `transport` of
/// `ZolanaWallet.open`.
class HttpClientTransport {
  final _client = HttpClient();

  /// Method and path of each request sent; the query, which can hold an
  /// API key, is left out.
  final sent = <String>[];

  Future<TransportResponse> send(TransportRequest request) async {
    final uri = Uri.parse(request.url);
    sent.add('${request.method} ${uri.path}');
    final outgoing = await _client.openUrl(request.method, uri);
    request.headers.forEach(outgoing.headers.set);
    outgoing.contentLength = request.body.length;
    outgoing.add(request.body);
    final response = await outgoing.close();
    final body = BytesBuilder(copy: false);
    await response.forEach(body.add);
    return TransportResponse(
      status: response.statusCode,
      body: body.takeBytes(),
    );
  }

  void close() => _client.close();
}
