export const SESSIONS = [
  {
    name: "Trabalho",
    initial: "T",
    auth: "global",
    state: "unlocked",
    count: 12,
  },
  {
    name: "Pessoal",
    initial: "P",
    auth: "global",
    state: "unlocked",
    count: 5,
  },
  { name: "Financeiro", initial: "F", auth: "own", state: "locked" },
  {
    name: "Projeto X",
    initial: "X",
    auth: "own",
    state: "locked",
    readOnly: true,
  },
] as const;

export const SECRETS = [
  {
    type: "password",
    typeLabel: "Senha",
    name: "GitHub",
    subtitle: "usuario@exemplo.com",
    username: "usuario@exemplo.com",
    password: "gh_mock_password_2026!",
    url: "https://github.com",
    notes: "Conta principal de desenvolvimento.",
    createdAt: "02/03/2026",
    updatedAt: "editado há 3 dias",
  },
  {
    type: "api-key",
    typeLabel: "API key",
    name: "Stripe (produção)",
    subtitle: "Ambiente de produção",
    key: "sk_live_mock_stripe_key",
    environment: "Produção",
    scopes: ["charges:read", "customers:read"],
    createdAt: "02/03/2026",
    updatedAt: "editado há 3 dias",
  },
  {
    type: "token",
    typeLabel: "Token",
    name: "Token CI/CD",
    subtitle: "Pipeline de deploy",
    value: "ci_mock_deploy_token",
    expiresAt: "31/12/2026",
    createdAt: "02/03/2026",
    updatedAt: "editado há 3 dias",
  },
  {
    type: "secure-note",
    typeLabel: "Nota secreta",
    name: "Recuperação da conta bancária",
    subtitle: "Códigos e instruções de recuperação",
    text: "Código de recuperação: MOCK-2026\nContato de emergência: suporte do banco.",
    createdAt: "02/03/2026",
    updatedAt: "editado há 3 dias",
  },
  {
    type: "ssh-key",
    typeLabel: "Chave SSH",
    name: "Servidor de deploy",
    subtitle: "deploy@servidor.exemplo.com",
    publicKey: "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5MOCK deploy@exemplo.com",
    privateKey: "-----BEGIN OPENSSH PRIVATE KEY-----\nmock-private-key\n-----END OPENSSH PRIVATE KEY-----",
    passphrase: "mock-deploy-passphrase",
    createdAt: "02/03/2026",
    updatedAt: "editado há 3 dias",
  },
] as const;

export type SessionFixture = (typeof SESSIONS)[number];
export type SecretFixture = (typeof SECRETS)[number];
