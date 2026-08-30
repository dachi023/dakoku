# dakoku

リポジトリごとに作業時間を記録する打刻CLI。

作業内容とその所要時間を、いたリポジトリに紐づけて記録します。リポジトリにクライアント名や案件名のラベルを設定しておくと、一週間の作業がそのまま案件別の時間として読み出せます。

```console
$ dakoku in "CLI実装"
▶ 09:30  A社ZZ案件 / CLI実装

$ dakoku out
■ 09:30 → 11:45  A社ZZ案件 / CLI実装  2h15m
```

## インストール

```console
$ cargo install --git https://github.com/dachi023/dakoku
```

## 打刻する

`dakoku in` は、いまいるリポジトリで作業を開始します。カレントディレクトリから親を辿って最も近い `.git` を探すため、サブディレクトリで打刻してもリポジトリルートに紐づきます。

```console
$ dakoku in "CLI実装"      # 開始
$ dakoku status            # 何を何時間やっているか
$ dakoku out               # 終了して作業時間を表示
```

同時に打刻できるセッションは1つです。打刻中に `dakoku in` するとエラーになります。`--switch` を付けたときだけ、実行中のものを新しい開始時刻で締めてから切り替えます。

```console
$ dakoku in "レビュー対応" --switch
■ 09:30 → 11:45  A社ZZ案件 / CLI実装  2h15m
▶ 11:45  A社ZZ案件 / レビュー対応
```

### 打刻し忘れたとき

`--at` で時刻を遡って指定できます。3つの形式を受け付けます。

| 形式 | 意味 |
| --- | --- |
| `--at 09:30` | 今日のその時刻。まだ未来なら前日と解釈する |
| `--at "2026-08-29 13:00"` | 日付と時刻を明示する（区切りは `T` でも可） |
| `--at -90m` | 現在から遡る。`-1h30m` や `-2h` も可 |

`dakoku edit` は打刻中のセッションを、締めたあとなら直前の記録を修正します。

```console
$ dakoku edit --note "設計レビュー"
$ dakoku edit --in 09:00 --out 11:30
```

## 記録を見る

`dakoku log` は今日の記録を表示します。`--week`、`--month`、`--since <DATE>`、`--all` で範囲を広げ、`--here` でいまのリポジトリだけに絞ります。

```console
$ dakoku log --week
2026-08-30 (日)
  09:30 → 11:45  2h15m  A社ZZ案件 / 設計
  22:16 → 00:06  1h50m  A社ZZ案件 / CLI実装
                 4h05m

2026-08-31 (月)
  00:06 → 00:46  40m    A社ZZ案件 / レビュー対応
  00:31 → 00:51  20m    playground / 調査
                 1h00m

A社ZZ案件   4h45m
playground  20m
─────────────────
合計        5h05m
```

記録は開始した日に属します。日をまたいだ作業も1行のまま扱われます。

## リポジトリにラベルを付ける

設定がない場合、記録にはリポジトリのディレクトリ名が使われます。`dakoku label` で案件名を紐づけられます。

```console
$ dakoku label set "A社ZZ案件"
~/ws/src/github.com/acme/api  A社ZZ案件

$ dakoku label show
~/ws/src/github.com/acme/api
  ラベル  A社ZZ案件

$ dakoku label list
$ dakoku label unset
```

`--path <PATH>` を付けると、いまいる場所以外のリポジトリを対象にできます。

ラベルは `~/.dakoku/settings.json` に保存されます。手で編集する前提の形式です。パスは最長一致で解決するので、親ディレクトリに付けたラベルは配下のリポジトリすべてに効きます。

```json
{
  "projects": {
    "~/ws/src/github.com/acme": { "label": "A社" },
    "~/ws/src/github.com/acme/api": { "label": "A社ZZ案件" }
  }
}
```

dakoku が知らないキーは書き戻すときも保持されるため、独自のメモを併記できます。

## データの置き場所

| ファイル | 内容 |
| --- | --- |
| `~/.dakoku/settings.json` | リポジトリごとのラベル |
| `~/.dakoku/current.json` | 打刻中のセッション |
| `~/.dakoku/entries.jsonl` | 締めた記録を1行ずつ追記 |

`DAKOKU_HOME` を設定すると保存先を変えられます。

締めた記録には、そのとき効いていたラベルを一緒に保存します。表示のたびに設定から引き直すわけではないため、あとで案件名を変えても過去の記録は当時のまま残ります。

## ライセンス

MIT または Apache-2.0 のデュアルライセンス。
