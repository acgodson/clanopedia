import { type ClassValue, clsx } from "clsx"
import { twMerge } from "tailwind-merge"
import { Principal } from "@dfinity/principal"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

export function formatPrincipal(principal: Principal | string, maxLength: number = 12): string {
  const principalStr = typeof principal === 'string' ? principal : principal.toString();
  if (principalStr.length <= maxLength) return principalStr;
  
  const halfLength = Math.floor((maxLength - 3) / 2);
  return `${principalStr.slice(0, halfLength)}...${principalStr.slice(-halfLength)}`;
}

export function formatPrincipalWithLabel(principal: Principal | string, label?: string): string {
  const formatted = formatPrincipal(principal);
  return label ? `${label} (${formatted})` : formatted;
} 