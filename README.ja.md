<!-- i18n: language-switcher -->
[English](README.md) | [日本語](README.ja.md)

# Snowflake コネクタ

Snowflake 用のネイティブ Irodori テーブルコネクタ拡張です。

このクレートは、Irodori 拡張マーケットプレイスで使用されるコネクタのメタデータ、ネイティブ ABI エクスポート、およびドライバ実装をパッケージ化しています。

## コネクタ

- 拡張 ID: `irodori.snowflake`
- エンジン ID: `snowflake`
- ワイヤープロトコル: `snowflake`
- デフォルトポート: `443`
- ネイティブ ABI: `irodori.connector.native.v1`
- ドライバ連携: `yes`
- マーケットプレイス公開範囲: `public`
- パッケージバージョン: `0.1.3`

パッケージには `db/snowflake.rs` からのデスクトップアダプタのソーススナップショットが含まれています。

コネクタのメタデータは `connector.config.json` と `irodori.extension.json` にあります。
Rust クレートは `src/lib.rs` からネイティブ ABI をエクスポートし、共有の JSON/バッファヘルパーに `irodori-connector-abi` を使用し、コネクタの動作は `src/driver.rs` に保持しています。

## 接続メタデータ

- エンドポイントモード: `hostPort`, `connectionString`
- トランスポートモード: `direct`, `sshTunnel`, `socks5Proxy`, `httpConnectProxy`, `proxyChain`
- TLS 対応: `yes`
- デフォルトで TLS 必須: `yes`
- カスタムドライバオプション: `yes`

### エンドポイントフィールド

| フィールド | ラベル | 型 | 必須 |
| --- | --- | --- | --- |
| `host` | ホスト | `string` | yes |
| `port` | ポート | `number` | no |
| `database` | データベース | `string` | no |

## 認証

コネクタはこれらの認証モードを宣伝しており、クライアントは適切な認証情報フィールドを表示できます。
ドライバ固有またはプロバイダ固有の値は必要に応じて `options` 経由で渡すことが可能です。

| 認証方法 | ラベル | 種類 | 秘密情報の用途 |
| --- | --- | --- | --- |
| `none` | 認証なし | `none` | なし |
| `connectionString` | 接続文字列 / DSN | `connectionString` | なし |
| `userPassword` | ユーザー/パスワード | `userPassword` | `password` |
| `snowflakeKeyPair` | キーペア JWT | `privateKey` | `privateKey`, `privateKeyPassphrase` |
| `oauth2` | OAuth 2.0 | `oauth2` | `token` |
| `accessToken` | アクセストークン | `token` | `token` |
| `snowflakeProgrammaticAccessToken` | プログラム的アクセストークン | `token` | `token` |
| `snowflakeSessionToken` | セッショントークン | `token` | `token` |
| `snowflakeWorkloadIdentity` | ワークロードアイデンティティフェデレーション | `iam` | `token` |
| `customDriverOptions` | カスタムドライバオプション | `custom` | `password`, `token`, `privateKey`, `privateKeyPassphrase` |

## エクスペリエンスメタデータ

- ドメイン: `warehouse`
- 結果ビュー: `worksheet`, `queryHistory`, `queryProfile`, `warehouseMonitor`, `costChart`, `copyReport`, `taskGraph`, `lineage`, `semanticModel`, `notebook`, `aiAssistant`, `table`
- オブジェクトタイプ: `accounts`, `databases`, `schemas`, `tables`, `views`, `semanticViews`, `stages`, `fileFormats`, `warehouses`, `roles`, `users`, `shares`, `tasks`, `streams`, `dynamicTables`, `notebooks`, `queryHistory`, `queryProfile`, `cortexFunctions`
- インスパイア元: Snowsight Worksheets, Snowsight Query History, Snowsight Query Profile, Snowflake SQL API, Snowflake Tasks, Snowflake Streams, Snowflake Dynamic Tables, Snowflake Semantic Views, Snowflake Cortex AISQL, sql-dialect-fmt

| ワークフロー | 結果ビュー | テンプレート |
| --- | --- | --- |
| ワークシートコンテキスト | `worksheet` | `snowflake-context` |
| クエリ履歴トリアージ | `queryHistory` | `snowflake-query-history`, `snowflake-expensive-queries`, `snowflake-result-scan` |
| クエリプロファイル詳細 | `queryProfile` | `snowflake-query-profile` |
| ウェアハウスモニター | `warehouseMonitor` | `snowflake-warehouse-load`, `snowflake-warehouse-metering` |
| ステージと COPY 制御 | `copyReport` | `snowflake-copy-validate`, `snowflake-copy-into`, `snowflake-load-history` |
| ストリーム、タスク、動的テーブル | `taskGraph` | `snowflake-stream-changes`, `snowflake-create-task`, `snowflake-dynamic-table` |
| セマンティックモデルスターター | `semanticModel` | `snowflake-semantic-view` |
| Cortex AISQL アシスタント | `aiAssistant` | `snowflake-cortex-complete`, `snowflake-cortex-summarize` |
| ロールと権限監査 | `table` | `snowflake-role-grants` |

| テンプレート | ラベル | 言語 | 結果ビュー |
| --- | --- | --- | --- |
| `snowflake-context` | 現在の Snowflake コンテキスト | `sql` | `worksheet` |
| `snowflake-query-history` | 最近のクエリ履歴 | `sql` | `queryHistory` |
| `snowflake-expensive-queries` | 高コストクエリ | `sql` | `costChart` |
| `snowflake-result-scan` | 結果スキャン | `sql` | `table` |
| `snowflake-query-profile` | クエリオペレータ統計 | `sql` | `queryProfile` |
| `snowflake-warehouse-load` | ウェアハウス負荷履歴 | `sql` | `warehouseMonitor` |
| `snowflake-warehouse-metering` | ウェアハウスメータリング履歴 | `sql` | `costChart` |
| `snowflake-copy-validate` | ステージファイル検証 | `sql` | `copyReport` |
| `snowflake-copy-into` | COPY INTO テーブル | `sql` | `copyReport` |
| `snowflake-load-history` | ロード履歴 | `sql` | `copyReport` |
| `snowflake-stream-changes` | ストリーム変更の読み取り | `sql` | `lineage` |
| `snowflake-create-task` | スケジュールタスク作成 | `sql` | `taskGraph` |
| `snowflake-dynamic-table` | 動的テーブル作成 | `sql` | `lineage` |
| `snowflake-semantic-view` | セマンティックビュー開始 | `sql` | `semanticModel` |
| `snowflake-cortex-complete` | Cortex 補完 | `sql` | `aiAssistant` |
| `snowflake-cortex-summarize` | Cortex 要約 | `sql` | `aiAssistant` |
| `snowflake-role-grants` | ロール権限 | `sql` | `table` |

## SQL 方言メタデータ

- 方言 ID: `snowflake.sql`
- 名前: Snowflake SQL
- 別名: `snowflake`, `snowsql`, `sfsql`, `Snowflake`
- キーワード数: `29`
- スニペット数: `5`

方言メタデータはハイライト、補完、スニペット、クエリテンプレートをサポートします。
完全な SQL パーサや構文木ではなく、意図的に軽量なメタデータです。

| スニペット | 種類 |
| --- | --- |
| sf context | `snippet` |
| sf query history | `snippet` |
| sf query profile | `snippet` |
| sf copy validate | `snippet` |
| sf cortex complete | `snippet` |

## ネイティブ ABI 呼び出し

| メソッド | レスポンス |
| --- | --- |
| `health` | コネクタのヘルス、エンジン ID、ABI バージョン、ドライバ状態を返します。 |
| `describe` | 埋め込みマニフェストとコネクタ設定を返します。 |
| `manifest` | 生の `irodori.extension.json` を返します。 |
| `config` | 生の `connector.config.json` を返します。 |
| `connect` | ネイティブコネクタ接続を開き、検証します。 |
| `query` | コネクタクエリを実行し、構造化された行または JSON 結果を返します。 |
| `metadata` | スキーマ、テーブル、カラム、インデックス、コレクション、または同等のメタデータを読み取ります。 |
| `close` | キャッシュされたネイティブ接続を閉じて削除します。 |

## 開発

このチェックアウト内のすべての拡張クレートは `../target` を共有しており、依存関係は兄弟リポジトリ間で一度だけコンパイルされます。

```sh
make check
make build
```

リリースパッケージはプラットフォーム固有のネイティブアーティファクトを `dist/native` に配置します。

## ライセンス

0BSD。ほぼあらゆる目的でこのプロジェクトを使用、コピー、修正、配布できます。