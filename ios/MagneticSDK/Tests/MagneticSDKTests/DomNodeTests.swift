import XCTest
@testable import MagneticSDK

final class DomNodeTests: XCTestCase {

    func testParseSimpleTextNode() throws {
        let json = #"{"tag":"h1","text":"Hello World"}"#
        let node = try parseDomNode(json)
        XCTAssertEqual(node.tag, "h1")
        XCTAssertEqual(node.text, "Hello World")
        XCTAssertNil(node.key)
        XCTAssertNil(node.children)
    }

    func testParseNodeWithKeyAndEvents() throws {
        let json = #"{"tag":"button","key":"btn","events":{"click":"increment"},"text":"+"}"#
        let node = try parseDomNode(json)
        XCTAssertEqual(node.tag, "button")
        XCTAssertEqual(node.key, "btn")
        XCTAssertEqual(node.event("click"), "increment")
        XCTAssertEqual(node.text, "+")
    }

    func testParseNodeWithAttrs() throws {
        let json = #"{"tag":"input","attrs":{"name":"email","type":"email","placeholder":"Enter email"}}"#
        let node = try parseDomNode(json)
        XCTAssertEqual(node.tag, "input")
        XCTAssertEqual(node.attr("name"), "email")
        XCTAssertEqual(node.attr("type"), "email")
        XCTAssertEqual(node.attr("placeholder"), "Enter email")
    }

    func testParseNodeWithChildren() throws {
        let json = """
        {
            "tag":"div",
            "key":"app",
            "children":[
                {"tag":"h1","text":"Count: 0"},
                {"tag":"button","events":{"click":"increment"},"text":"+"}
            ]
        }
        """
        let node = try parseDomNode(json)
        XCTAssertEqual(node.tag, "div")
        XCTAssertEqual(node.key, "app")
        XCTAssertEqual(node.childNodes.count, 2)
        XCTAssertEqual(node.childNodes[0].tag, "h1")
        XCTAssertEqual(node.childNodes[0].text, "Count: 0")
        XCTAssertEqual(node.childNodes[1].event("click"), "increment")
    }

    func testParseDeeplyNestedTree() throws {
        let json = """
        {
            "tag":"div",
            "children":[{
                "tag":"form",
                "events":{"submit":"add_item"},
                "children":[
                    {"tag":"input","attrs":{"name":"title","placeholder":"Enter..."}},
                    {"tag":"button","attrs":{"type":"submit"},"text":"Add"}
                ]
            }]
        }
        """
        let node = try parseDomNode(json)
        let form = node.childNodes[0]
        XCTAssertEqual(form.tag, "form")
        XCTAssertEqual(form.event("submit"), "add_item")
        XCTAssertEqual(form.childNodes.count, 2)
        XCTAssertEqual(form.childNodes[0].attr("name"), "title")
    }

    func testCssClass() {
        let node = DomNode(tag: "div", attrs: ["class": "row flex-row"])
        XCTAssertEqual(node.cssClass, "row flex-row")
    }

    func testIsRowLayout() {
        XCTAssertTrue(DomNode(tag: "div", attrs: ["class": "row"]).isRowLayout)
        XCTAssertTrue(DomNode(tag: "div", attrs: ["class": "flex-row"]).isRowLayout)
        XCTAssertTrue(DomNode(tag: "nav").isRowLayout)
        XCTAssertFalse(DomNode(tag: "div").isRowLayout)
        XCTAssertFalse(DomNode(tag: "div", attrs: ["class": "column"]).isRowLayout)
    }

    func testUnknownKeysIgnored() throws {
        let json = #"{"tag":"div","unknownField":"whatever","text":"ok"}"#
        let node = try parseDomNode(json)
        XCTAssertEqual(node.tag, "div")
        XCTAssertEqual(node.text, "ok")
    }

    func testParseSnapshot() throws {
        let json = """
        {
            "root": {
                "tag": "div",
                "key": "app",
                "children": [
                    {"tag": "h1", "text": "Count: 0"},
                    {"tag": "button", "events": {"click": "increment"}, "text": "+"}
                ]
            }
        }
        """
        let snapshot = try parseSnapshot(json)
        XCTAssertEqual(snapshot.root.tag, "div")
        XCTAssertEqual(snapshot.root.key, "app")
        XCTAssertEqual(snapshot.root.childNodes.count, 2)
    }

    func testParseRealServerSnapshot() throws {
        let json = """
        {
            "tag":"div","key":"app","attrs":{"class":"task-board"},
            "children":[
                {"tag":"magnetic:head","children":[{"tag":"title","text":"My Todos"}]},
                {"tag":"h1","key":"title","text":"My Todos"},
                {"tag":"form","key":"form","events":{"submit":"add_todo"},"children":[
                    {"tag":"input","key":"input","attrs":{"name":"text","placeholder":"What needs to be done?"}},
                    {"tag":"button","key":"submit","attrs":{"type":"submit"},"text":"Add"}
                ]},
                {"tag":"ul","key":"list","children":[
                    {"tag":"li","key":"todo-1","children":[
                        {"tag":"span","text":"Buy groceries"},
                        {"tag":"button","events":{"click":"toggle_1"},"text":"✓"},
                        {"tag":"button","events":{"click":"delete_1"},"text":"×"}
                    ]}
                ]}
            ]
        }
        """
        let node = try parseDomNode(json)
        XCTAssertEqual(node.tag, "div")
        XCTAssertEqual(node.cssClass, "task-board")
        XCTAssertEqual(node.childNodes.count, 4)

        // magnetic:head should parse but renderer skips it
        XCTAssertEqual(node.childNodes[0].tag, "magnetic:head")

        // Form
        let form = node.childNodes[2]
        XCTAssertEqual(form.event("submit"), "add_todo")

        // List
        let list = node.childNodes[3]
        XCTAssertEqual(list.tag, "ul")
        let todoItem = list.childNodes[0]
        XCTAssertEqual(todoItem.key, "todo-1")
        XCTAssertEqual(todoItem.childNodes[1].event("click"), "toggle_1")
        XCTAssertEqual(todoItem.childNodes[2].event("click"), "delete_1")
    }
}
