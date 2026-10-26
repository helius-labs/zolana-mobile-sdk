import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zolana_mobile_demo/action_sheet.dart';
import 'package:zolana_mobile_demo/demo_assets.dart';
import 'package:zolana_mobile_demo/demo_keys.dart';

void main() {
  testWidgets('a payment plus its fee must fit the local proof limit', (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: ActionSheet(
            action: WalletAction.send,
            asset: sol,
            available: BigInt.from(10000000000),
            limit: BigInt.from(8000000000),
            suggestedRecipient: demoAccounts.first,
            fee: (_) => BigInt.from(1000000000),
            run: (_, _) async => throw StateError('Must merge first'),
          ),
        ),
      ),
    );
    await tester.enterText(find.byType(TextField).last, '8');
    await tester.pumpAndSettle();
    expect(
      find.text('Merge notes with the TEE first to send this amount'),
      findsOneWidget,
    );
    expect(
      tester.widget<FilledButton>(find.byType(FilledButton)).onPressed,
      isNull,
    );
    await tester.enterText(find.byType(TextField).last, '7');
    await tester.pumpAndSettle();
    expect(
      tester.widget<FilledButton>(find.byType(FilledButton)).onPressed,
      isNotNull,
    );
  });
}
