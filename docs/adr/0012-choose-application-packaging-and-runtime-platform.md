# 0012: アプリケーションの配布単位と実行基盤

- 日付: 2026-09-06
- 意思決定者: プロジェクト
- 関連 ADR: 0001, 0006, 0011

## コンテキスト

API、migration、PostgreSQLを稼働させる具体的なmanaged platformと、環境間で昇格する配布単位を決めます。

## 用語

| 用語 | 定義 |
| --- | --- |
| managed platform | server 管理の一部を cloud provider が担う実行基盤。 |
| OCI image | アプリケーションと実行に必要な file をまとめた標準的な配布単位。 |
| digest | image の内容を一意に特定する固定長の値。 |
| migration runner | DB の migration を適用するためだけに実行する処理。 |
| stateless process | 終了後も保持すべき状態を自身の実行領域に保存しない process。 |
| SBOM | image に含まれる software component の一覧。 |

## 決定

- APIをGoogle Cloud Run service、migration runnerをGoogle Cloud Run Jobで実行します。
- OCI imageをGoogle Artifact Registryに保存し、digestを固定した同一imageを環境間で昇格します。
- PostgreSQLはGoogle Cloud SQL for PostgreSQLを使用し、Cloud Runからprivate IPで接続します。
- APIとmigration runnerは同じRust workspaceの別entry pointとし、API起動時にmigrationを実行しません。
- containerはnon-rootのstateless processとし、secretはGoogle Secret Managerから実行時に渡します。imageの署名、SBOM生成、脆弱性更新を行います。

## 検討

### D-1: アプリケーションの配布単位

#### 判断基準

- 同じartifactを環境間で再buildせず昇格できること
- localとproductionの差を抑えられること
- 選定するmanaged platformで直接実行できること

#### 選択肢

- **OCI image:** image管理は必要だが、標準化され実行先が多い。
- **Functions package:** packageは小さいが、runtimeとrequest modelの制約が強い。
- **VMへのbinary配布:** 単純だが、OS環境と更新を個別管理する。

#### 採用

**OCI image**を採用し、registry上のdigestで実行物を一意に指定します。

### D-2: managed container実行基盤

#### 判断基準

- HTTP serviceとone-shot jobの両方をmanaged serviceとして実行できること
- 小規模時に常時稼働nodeを管理しなくてよいこと
- managed PostgreSQLへprivate networkで接続できること
- OCI imageをそのまま配置できること

#### 選択肢

- **Google Cloud Run:** serviceとJobを提供し、server管理を減らせるが、Google Cloudへ依存する。
- **AWS ECS on Fargate:** serviceとtaskを実行できるが、networkや周辺resourceの構成項目が多い。
- **Azure Container Apps:** serviceとjobを実行できるが、Azure固有の環境構成へ依存する。
- **Managed Kubernetes:** 制御範囲は広いが、初期段階にはcluster運用と構成が過剰になる。

#### 採用

**Google Cloud Run**を採用します。APIはCloud Run service、migrationはCloud Run Jobとして実行します。

### D-3: image registryと昇格方法

#### 判断基準

- Cloud Runと同じcloud内でimageを管理できること
- tagの付け替えで実行物が変わらないこと
- buildとdeployを分離できること

#### 選択肢

- **Google Artifact Registryとdigest:** Google Cloudへ依存するが、imageを不変に指定できる。
- **GitHub Container Registryとtag:** repositoryとの統合は容易だが、Google Cloudとの認証とtag不変性を別途管理する。
- **環境ごとの再build:** 構成は単純だが、環境間で同一artifactを保証できない。

#### 採用

**Google Artifact Registryとdigest**を採用します。一度buildしたimage digestを環境間で昇格し、環境ごとに再buildしません。

### D-4: PostgreSQLのmanaged productと接続

#### 判断基準

- PostgreSQLの固定major versionをmanaged serviceで運用できること
- Cloud Runからpublic internetを経由せず接続できること
- backup、patch、可用性設定をservice側で管理できること

#### 選択肢

- **Google Cloud SQL for PostgreSQL:** Cloud Runと統合しやすいが、Google Cloudへ依存する。
- **自己管理PostgreSQL:** 制御範囲は広いが、backup、patch、failoverを管理する必要がある。
- **外部managed PostgreSQL:** providerを分離できるが、network、認証、障害切り分けが複雑になる。

#### 採用

**Google Cloud SQL for PostgreSQL**を採用し、Cloud Runからprivate IPで接続します。

### D-5: migrationの実行形態

#### 判断基準

- APIのscale-outとmigration実行回数を分離できること
- migration失敗時にAPIの起動を巻き込まないこと
- 同じsourceとimageを共有できること

#### 選択肢

- **Cloud Run Jobの別entry point:** 明示的な実行が必要だが、一回限りの処理として管理できる。
- **API起動時に実行:** deployは単純だが、複数instanceによる競合とrollbackが複雑になる。
- **CI runnerからDBへ接続:** 実行は集中するが、CIへDB network accessを与える必要がある。

#### 採用

**Cloud Run Jobの別entry point**を採用します。APIと同じRust workspaceおよびimageから実行しますが、API起動時にはmigrationを実行しません。

### D-6: container設定とsecret

#### 判断基準

- imageへsecretや環境固有値を含めないこと
- processの権限を限定すること
- Cloud Runのinstance交換に依存しないこと

#### 選択肢

- **stateless containerとSecret Manager:** 外部設定が必要だが、imageを環境間で共有できる。
- **設定とsecretをimageへ埋め込む:** 起動は簡単だが、環境ごとのimageと漏えいriskが生じる。
- **永続volumeへ状態を保存:** process内では扱いやすいが、instance交換とscale-outに適さない。

#### 採用

**stateless containerとGoogle Secret Manager**を採用します。containerをnon-rootで実行し、secretを実行時に注入します。

## 影響

Google Cloud、Cloud Run、Artifact Registry、Cloud SQL、Secret ManagerのIAM、network、quota、cost、障害特性へ依存します。image supply chainと各productの構成管理が増えます。
