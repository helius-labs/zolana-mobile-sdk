import 'package:build_tool/src/download.dart';
import 'package:http/http.dart';
import 'package:http/testing.dart';
import 'package:test/test.dart';

void main() {
  final url = Uri.parse('https://example.invalid/binary');

  Future<(Response, int)> download(List<Object> replies) async {
    var calls = 0;
    final client = MockClient((_) async {
      final reply = replies[calls++];
      if (reply is Exception) throw reply;
      return Response('body', reply as int);
    });
    final response =
        await getWithRetry(url, client: client, delay: (_) => Duration.zero);
    return (response, calls);
  }

  test('retries dropped connections and server errors', () async {
    final (response, calls) =
        await download([ClientException('connection closed'), 502, 429, 200]);
    expect(response.statusCode, 200);
    expect(calls, 4);
  });

  test('returns other statuses at once', () async {
    final (response, calls) = await download([404]);
    expect(response.statusCode, 404);
    expect(calls, 1);
  });

  test('gives up after the last attempt', () async {
    final (response, calls) =
        await download(List.filled(downloadAttempts, 503));
    expect(response.statusCode, 503);
    expect(calls, downloadAttempts);
    await expectLater(
      download(List.filled(downloadAttempts, ClientException('reset'))),
      throwsA(isA<ClientException>()),
    );
  });
}
