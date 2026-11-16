// 记一笔的金额键盘：输入的是连续加减的算式，每一项按 `yuanCents` 转成分再算。
import { yuanCents } from "./format";

/** 算式的结果，分；不完整（如结尾是运算符）为空。 */
export function amountResult(expression: string): number | null {
  const parts = expression.split(/([+-])/);
  let total = 0;
  for (let i = 0; i < parts.length; i += 2) {
    const cents = yuanCents(parts[i]);
    if (cents === null) return null;
    total += i && parts[i - 1] === "-" ? -cents : cents;
  }
  return Number.isSafeInteger(total) ? total : null;
}

/** 按一个键之后的算式。 */
export function amountInput(expression: string, key: string): string {
  if (key === "back") return expression.slice(0, -1);
  if (key === "+" || key === "-") {
    if (!expression) return expression;
    return /[+-]$/.test(expression) ? expression.slice(0, -1) + key : expression + key;
  }
  const operand = expression.split(/[+-]/).at(-1)!;
  if (key === ".") return operand.includes(".") ? expression : expression + (operand ? "." : "0.");
  if (operand.includes(".") && operand.split(".")[1].length === 2) return expression;
  if (operand.replace(".", "").length >= 11) return expression;
  return operand === "0" ? expression.slice(0, -1) + key : expression + key;
}
