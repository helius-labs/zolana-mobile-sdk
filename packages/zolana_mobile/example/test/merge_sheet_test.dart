import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zolana_mobile_demo/demo_assets.dart';
import 'package:zolana_mobile_demo/merge_sheet.dart';

void main() {
  testWidgets('opt in before merging, block duplicate actions, then opt out', (
    tester,
  ) async {
    final enabled = <bool>[];
    final proof = Completer<String>();
    var merges = 0;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: MergeSheet(
            asset: sol,
            notes: BigInt.from(9),
            readEnabled: () async => false,
            setEnabled: (value) async {
              enabled.add(value);
              return 'setting';
            },
            merge: () {
              merges++;
              return proof.future;
            },
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    final merge = find.widgetWithText(FilledButton, 'Merge with TEE');
    expect(tester.widget<FilledButton>(merge).onPressed, isNull);
    await tester.tap(find.byType(Switch));
    await tester.pumpAndSettle();
    expect(enabled, [true]);
    await tester.tap(merge);
    await tester.pump();
    expect(merges, 1);
    expect(tester.widget<FilledButton>(merge).onPressed, isNull);
    expect(tester.widget<Switch>(find.byType(Switch)).onChanged, isNull);
    proof.complete('merged');
    await tester.pumpAndSettle();
    expect(
      find.text('Notes merged. Your private balance is unchanged.'),
      findsOneWidget,
    );
    expect(find.text('Copy explorer link'), findsOneWidget);
    await tester.tap(find.byType(Switch));
    await tester.pumpAndSettle();
    expect(enabled, [true, false]);
  });

  testWidgets('a failed opt-in does not enable merges', (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: MergeSheet(
            asset: sol,
            notes: BigInt.two,
            readEnabled: () async => false,
            setEnabled: (_) async => throw StateError('Setting was rejected'),
            merge: () async => throw StateError('Must not merge'),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.byType(Switch));
    await tester.pumpAndSettle();
    expect(tester.widget<Switch>(find.byType(Switch)).value, isFalse);
    expect(
      tester
          .widget<FilledButton>(
            find.widgetWithText(FilledButton, 'Merge with TEE'),
          )
          .onPressed,
      isNull,
    );
    expect(find.textContaining('Setting was rejected'), findsOneWidget);
  });

  testWidgets('one note cannot be merged', (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: MergeSheet(
            asset: sol,
            notes: BigInt.one,
            readEnabled: () async => true,
            setEnabled: (_) async => null,
            merge: () async => throw StateError('Must not merge'),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(
      find.text('At least two notes are needed to merge.'),
      findsOneWidget,
    );
    expect(
      tester
          .widget<FilledButton>(
            find.widgetWithText(FilledButton, 'Merge with TEE'),
          )
          .onPressed,
      isNull,
    );
  });
}
