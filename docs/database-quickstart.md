# 数据库快速配置指南

本文档提供 **faasd-in-rust** 项目数据库的快速配置步骤。

## 🚀 快速开始

### 1. 安装 PostgreSQL

```bash
# Ubuntu/Debian
sudo apt update && sudo apt install postgresql postgresql-contrib libpq-dev

# macOS
brew install postgresql && brew services start postgresql

# nixos
自己没手吗？

# 检查安装
psql --version
```
### 2. 创建数据库

```bash
# 进入 PostgreSQL
sudo -u postgres psql

# 执行以下 SQL 命令
CREATE USER dragonos WITH PASSWORD 'dragonos';
ALTER USER dragonos WITH SUPERUSER;
CREATE DATABASE faasd_rs_db OWNER dragonos;
CREATE DATABASE diesel_demo_db_dragonos OWNER dragonos;
\q
```

### 3. 安装 Diesel CLI

```bash
cargo install diesel_cli --no-default-features --features postgres
```

### 4. 配置环境变量

根据你填的密码修改项目根目录的 `.env` 文件，确保包含：

```bash
export DATABASE_URL=postgres://用户名:密码@localhost/数据库名字
export DATABASE_URL=postgres://dragonos:dragonos@localhost/faasd_rs_db
export TEST_DATABASE_URL=postgres://dragonos:dragonos@localhost/diesel_demo_db_dragonos
JWT_SECRET="HelloRust"
```

### 5. 运行数据库迁移

```bash
# 在项目根目录执行
export DATABASE_URL=postgres://dragonos:dragonos@localhost/faasd_rs_db
diesel migration run 
export DATABASE_URL=postgres://dragonos:dragonos@localhost/diesel_demo_db_dragonos
diesel migration run 

```

### 6.🔧 验证安装

运行以下命令验证配置：

```bash
# 检查迁移状态
diesel migration list

```bash
❯ psql postgres://dragonos:dragonos@localhost/faasd_rs_db

psql (14.18 (Ubuntu 14.18-0ubuntu0.22.04.1))
SSL connection (protocol: TLSv1.3, cipher: TLS_AES_256_GCM_SHA384, bits: 256, compression: off)
Type "help" for help.

faasd_rs_db=# \dt
                   List of relations
 Schema |            Name            | Type  |  Owner   
--------+----------------------------+-------+----------
 public | __diesel_schema_migrations | table | dragonos
 public | users                      | table | dragonos
(2 rows)

faasd_rs_db=# \d users
                                  Table "public.users"
    Column     |            Type             | Collation | Nullable |      Default      
---------------+-----------------------------+-----------+----------+-------------------
 uid           | uuid                        |           | not null | gen_random_uuid()
 username      | character varying           |           | not null | 
 password_hash | character varying           |           | not null | 
 created_at    | timestamp without time zone |           | not null | CURRENT_TIMESTAMP
Indexes:
    "users_pkey" PRIMARY KEY, btree (uid)
    "users_username_key" UNIQUE CONSTRAINT, btree (username)
```

然后就可以运行项目了~~~