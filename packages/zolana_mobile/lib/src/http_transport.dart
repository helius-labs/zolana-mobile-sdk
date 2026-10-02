import 'package:http/http.dart' as http;

import 'rust/third_party/zolana_mobile.dart'
    show TransportRequest, TransportResponse;

/// The wallet's transport when the application passes none: `package:http`,
/// one connection pool per wallet, closed with it.
///
/// It sends each request as it is. A plaintext (`http`) URL off loopback is
/// refused unless [allowInsecureHttp] is set, and a request fails when its
/// response stalls for 30 seconds.
class HttpTransport {
  HttpTransport({required this.allowInsecureHttp, http.Client? client})
    : _client = client ?? http.Client();

  final bool allowInsecureHttp;
  final http.Client _client;

  static const _stall = Duration(seconds: 30);

  Future<TransportResponse> send(TransportRequest request) async {
    final url = Uri.parse(request.url);
    if (!allowInsecureHttp && !_secure(url)) throw _InsecureUrl(url);
    final outgoing = http.Request(request.method, url)
      ..headers.addAll(request.headers)
      ..bodyBytes = request.body;
    final response = await _client.send(outgoing).timeout(_stall);
    final body = http.ByteStream(response.stream.timeout(_stall));
    return TransportResponse(
      status: response.statusCode,
      body: await body.toBytes(),
    );
  }

  void close() => _client.close();
}

/// `https`, or `http` to this device.
bool _secure(Uri url) =>
    url.scheme == 'https' ||
    (url.scheme == 'http' &&
        const {'localhost', '127.0.0.1', '::1'}.contains(url.host));

class _InsecureUrl implements Exception {
  const _InsecureUrl(this.url);

  final Uri url;

  @override
  String toString() =>
      '${url.scheme}://${url.host} is plaintext off loopback; '
      'WalletConfig.allowInsecureHttp allows it for a test cluster';
}
