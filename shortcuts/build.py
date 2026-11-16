"""生成并签名快捷指令「记一笔」。

用法：python3 shortcuts/build.py [base_url]
  base_url 默认 http://127.0.0.1:8730；token 读自 config.toml。
产物：shortcuts/记一笔.shortcut（含 token，不入库），双击导入。
"""

import plistlib
import subprocess
import sys
import tempfile
import tomllib
import uuid
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "shortcuts" / "记一笔.shortcut"
OBJ = "￼"  # 变量占位符

actions: list[dict] = []


def new_uuid() -> str:
    return str(uuid.uuid4()).upper()


def act(identifier: str, **params) -> str:
    """追加动作，返回其输出 UUID。"""
    uid = params.setdefault("UUID", new_uuid())
    actions.append(
        {
            "WFWorkflowActionIdentifier": f"is.workflow.actions.{identifier}",
            "WFWorkflowActionParameters": params,
        }
    )
    return uid


# —— 参数序列化 ——


def var(name: str) -> dict:
    return {"Type": "Variable", "VariableName": name}


def out(uid: str, name: str) -> dict:
    return {"Type": "ActionOutput", "OutputUUID": uid, "OutputName": name}


def attach(ref: dict) -> dict:
    return {"Value": ref, "WFSerializationType": "WFTextTokenAttachment"}


def text(*parts) -> dict:
    """文本与变量混排：parts 为 str 或 var()/out() 引用。"""
    s, ranges = "", {}
    for p in parts:
        if isinstance(p, str):
            s += p
        else:
            ranges[f"{{{len(s)}, 1}}"] = p
            s += OBJ
    value = {"string": s}
    if ranges:
        value["attachmentsByRange"] = ranges
    return {"Value": value, "WFSerializationType": "WFTextTokenString"}


def file_value(ref: dict) -> dict:
    """表单「文件」类型的值；直接用 attach() 会在导入时被丢弃。"""
    return {"Value": attach(ref), "WFSerializationType": "WFTokenAttachmentParameterState"}


def fields(items: list[tuple[str, dict, int]]) -> dict:
    return {
        "Value": {
            "WFDictionaryFieldValueItems": [
                {"WFItemType": t, "WFKey": text(k), "WFValue": v} for k, v, t in items
            ]
        },
        "WFSerializationType": "WFDictionaryFieldValue",
    }


# —— 常用动作 ——


def set_var(name: str, ref: dict) -> None:
    act("setvariable", WFVariableName=name, WFInput=attach(ref))


def get_key(dict_var: str, key: str) -> str:
    return act(
        "getvalueforkey",
        WFInput=attach(var(dict_var)),
        WFGetDictionaryValueType="Value",
        WFDictionaryKey=key,
    )


def get_key_to(dict_var: str, key: str, name: str) -> None:
    set_var(name, out(get_key(dict_var, key), "词典值"))


def post(path: str, *, form=None, json=None) -> str:
    params = dict(
        WFURL=text(var("base"), path),
        WFHTTPMethod="POST",
        ShowHeaders=True,
        WFHTTPHeaders=fields([("Authorization", text("Bearer ", var("token")), 0)]),
    )
    if form is not None:
        params.update(WFHTTPBodyType="Form", WFFormValues=fields(form))
    if json is not None:
        params.update(WFHTTPBodyType="JSON", WFJSONValues=fields(json))
    return act("downloadurl", **params)


IS, CONTAINS = 4, 99


def if_(subject: str, condition: int, value: str, then, otherwise=None) -> None:
    """如果 <变量 subject 转文本> <condition> <value>；格式取自 App 导出。"""
    gid = new_uuid()
    as_text = var(subject) | {
        "Aggrandizements": [
            {
                "Type": "WFCoercionVariableAggrandizement",
                "CoercionItemClass": "WFStringContentItem",
            }
        ]
    }
    act(
        "conditional",
        GroupingIdentifier=gid,
        WFControlFlowMode=0,
        WFCondition=condition,
        WFConditionalActionString=value,
        WFInput={"Type": "Variable", "Variable": attach(as_text)},
    )
    then()
    if otherwise:
        act("conditional", GroupingIdentifier=gid, WFControlFlowMode=1)
        otherwise()
    act("conditional", GroupingIdentifier=gid, WFControlFlowMode=2)


def if_type_is(value: str, body) -> None:
    if_("type", IS, value, body)


def notify_and_exit(body: dict) -> None:
    act("notification", WFNotificationActionTitle="记一笔", WFNotificationActionBody=body)
    act("exit")


def ask(input_type: str) -> str:
    default = get_key("resp", "default")
    return act(
        "ask",
        WFInputType=input_type,
        WFAskActionPrompt=text(var("summary")),
        WFAskActionDefaultAnswer=text(out(default, "词典值")),
    )


def choose(multiple: bool) -> str:
    options = get_key("resp", "options")
    return act(
        "choosefromlist",
        WFInput=attach(out(options, "词典值")),
        WFChooseFromListActionPrompt=text(var("summary")),
        WFChooseFromListActionSelectMultiple=multiple,
    )


# —— 流程，对应 shortcuts/README.md ——


def build(base: str, token: str) -> None:
    set_var("base", out(act("gettext", WFTextActionText=base), "文本"))
    set_var("token", out(act("gettext", WFTextActionText=token), "文本"))
    shot = act("takescreenshot")
    resp = post("/recognize", form=[("image", file_value(out(shot, "截屏")), 5)])
    set_var("resp", out(resp, "URL的内容"))

    loop = new_uuid()
    act("repeat.count", GroupingIdentifier=loop, WFControlFlowMode=0, WFRepeatCount=50)

    # 响应不是服务端的统一 JSON（反代错误页、404 等）时，原样提示，避免「无法转换为字典」。
    if_(
        "resp",
        CONTAINS,
        '"type"',
        lambda: None,
        lambda: notify_and_exit(text("服务异常：", var("resp"))),
    )
    get_key_to("resp", "type", "type")
    get_key_to("resp", "summary", "summary")

    def done():
        notify_and_exit(text(out(get_key("resp", "message"), "词典值")))

    def choose_one():
        set_var("value", out(choose(False), "所选项目"))

    def choose_many():
        picked = choose(True)
        joined = act(
            "text.combine",
            text=attach(out(picked, "所选项目")),
            WFTextSeparator="New Lines",
        )
        set_var("value", out(joined, "合并的文本"))

    def ask_number():
        set_var("value", out(ask("Number"), "提供的输入"))

    def ask_text():
        set_var("value", out(ask("Text"), "提供的输入"))

    def ask_date():
        picked = ask("Date and Time")
        iso = act(
            "format.date",
            WFDate=text(out(picked, "提供的输入")),
            WFDateFormatStyle="ISO 8601",
            WFISO8601IncludeTime=True,
        )
        set_var("value", out(iso, "格式化的日期"))

    if_type_is("done", done)
    if_type_is("choose", choose_one)
    if_type_is("choose_many", choose_many)
    if_type_is("number", ask_number)
    if_type_is("text", ask_text)
    if_type_is("date", ask_date)

    get_key_to("resp", "draft", "draft")
    get_key_to("resp", "field", "field")
    resp = post(
        "/step",
        json=[
            ("draft", text(var("draft")), 0),
            ("field", text(var("field")), 0),
            ("value", text(var("value")), 0),
        ],
    )
    set_var("resp", out(resp, "URL的内容"))

    act("repeat.count", GroupingIdentifier=loop, WFControlFlowMode=2)


def main() -> None:
    base = sys.argv[1] if len(sys.argv) > 1 else "http://127.0.0.1:8730"
    token = tomllib.loads((ROOT / "config.toml").read_text())["token"]
    build(base.rstrip("/"), token)
    workflow = {
        "WFWorkflowActions": actions,
        "WFWorkflowClientVersion": "5114.0.6.204",
        "WFWorkflowMinimumClientVersion": 900,
        "WFWorkflowMinimumClientVersionString": "900",
        "WFWorkflowIcon": {
            "WFWorkflowIconGlyphNumber": 61440,
            "WFWorkflowIconStartColor": -1263359489,
        },
        "WFWorkflowImportQuestions": [],
        "WFWorkflowInputContentItemClasses": [],
        "WFWorkflowOutputContentItemClasses": [],
        "WFWorkflowTypes": [],
        "WFQuickActionSurfaces": [],
        "WFWorkflowHasOutputFallback": False,
        "WFWorkflowHasShortcutInputVariables": False,
    }
    with tempfile.TemporaryDirectory() as tmp:
        unsigned = Path(tmp) / "记一笔.shortcut"
        unsigned.write_bytes(plistlib.dumps(workflow, fmt=plistlib.FMT_BINARY))
        subprocess.run(
            ["shortcuts", "sign", "--mode", "anyone", "-i", unsigned, "-o", OUT],
            check=True,
        )
    print(f"已生成 {OUT}（{len(actions)} 个动作，base={base}）")


if __name__ == "__main__":
    main()
