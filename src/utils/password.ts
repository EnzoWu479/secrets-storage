export type StrengthLevel = 1 | 2 | 3 | 4;

export interface PasswordStrengthResult {
  level: StrengthLevel;
  label: string;
}

const LABELS: Record<StrengthLevel, string> = {
  1: "Fraca",
  2: "Média",
  3: "Boa",
  4: "Forte",
};

// Heurística simples de UI (NÃO é uma medida de segurança real): pontua por
// comprimento e variedade de classes de caractere. O PasswordStrength espera
// um nível de 1 a 4, então o mínimo é sempre 1.
export function passwordStrength(password: string): PasswordStrengthResult {
  let score = 0;
  if (password.length >= 8) score++;
  if (password.length >= 12) score++;
  if (/[a-z]/.test(password) && /[A-Z]/.test(password)) score++;
  if (/\d/.test(password)) score++;
  if (/[^A-Za-z0-9]/.test(password)) score++;

  const level = (Math.min(4, Math.max(1, score)) as StrengthLevel);
  return { level, label: LABELS[level] };
}
