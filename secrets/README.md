# Secret storage with SOPS

The repository keeps production/staging environment files encrypted at rest
using [Mozilla SOPS](https://github.com/getsops/sops) and a Google Cloud KMS
key. This lets us review, version and share `.env` style files without
committing cleartext secrets.

## 1. Prepare the KMS key

Run once per project:

```bash
export PROJECT_ID=vpass-477522
export LOCATION=asia-east1
export KEYRING=vpass-secrets
export KEY_NAME=app-config

gcloud kms keyrings create ${KEYRING} \
  --project=${PROJECT_ID} \
  --location=${LOCATION}

gcloud kms keys create ${KEY_NAME} \
  --project=${PROJECT_ID} \
  --location=${LOCATION} \
  --keyring=${KEYRING} \
  --purpose=encryption
```

If the key (or key ring) already exists, the commands will report that and
nothing else is required. The path must match the entry in `.sops.yaml`.

## 2. Create / edit an encrypted env file

1. Install `sops` locally.
2. Populate a plaintext `.env` based on `.env.example`.
3. Encrypt it:

```bash
cp .env secrets/prod.env
sops --encrypt --in-place secrets/prod.env
mv secrets/prod.env secrets/prod.env.enc
```

After the first encryption you can simply run `sops secrets/prod.env.enc` to
edit the file in place; it will decrypt into your editor and re-encrypt on save.

## 3. Decrypt for local use

When you need the secrets locally:

```bash
scripts/decrypt-env.sh prod
source secrets/.prod.env
```

The helper script writes a temporary `secrets/.prod.env` (git-ignored). Remove it
when finished.

## 4. Cloud Build / deploy usage

During CI/CD, the Cloud Build service account must have
`roles/cloudkms.cryptoKeyDecrypter` on the KMS key. The build step can then run:

```bash
sops --decrypt secrets/prod.env.enc > secrets/.prod.env
```

Use the decrypted values to update Secret Manager or to pass `--set-env-vars`
when deploying to Cloud Run.
