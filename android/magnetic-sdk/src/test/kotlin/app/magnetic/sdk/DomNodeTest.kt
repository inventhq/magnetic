package app.magnetic.sdk

import org.junit.Assert.*
import org.junit.Test

class DomNodeTest {

    @Test
    fun `parse simple text node`() {
        val json = """{"tag":"h1","text":"Hello World"}"""
        val node = parseDomNode(json)
        assertEquals("h1", node.tag)
        assertEquals("Hello World", node.text)
        assertNull(node.key)
        assertNull(node.children)
    }

    @Test
    fun `parse node with key and events`() {
        val json = """{"tag":"button","key":"btn","events":{"click":"increment"},"text":"+"}"""
        val node = parseDomNode(json)
        assertEquals("button", node.tag)
        assertEquals("btn", node.key)
        assertEquals("increment", node.event("click"))
        assertEquals("+", node.text)
    }

    @Test
    fun `parse node with attrs`() {
        val json = """{"tag":"input","attrs":{"name":"email","type":"email","placeholder":"Enter email"}}"""
        val node = parseDomNode(json)
        assertEquals("input", node.tag)
        assertEquals("email", node.attr("name"))
        assertEquals("email", node.attr("type"))
        assertEquals("Enter email", node.attr("placeholder"))
    }

    @Test
    fun `parse node with children`() {
        val json = """{
            "tag":"div",
            "key":"app",
            "children":[
                {"tag":"h1","text":"Count: 0"},
                {"tag":"button","events":{"click":"increment"},"text":"+"}
            ]
        }"""
        val node = parseDomNode(json)
        assertEquals("div", node.tag)
        assertEquals("app", node.key)
        assertNotNull(node.children)
        assertEquals(2, node.children!!.size)
        assertEquals("h1", node.children!![0].tag)
        assertEquals("Count: 0", node.children!![0].text)
        assertEquals("increment", node.children!![1].event("click"))
    }

    @Test
    fun `parse deeply nested tree`() {
        val json = """{
            "tag":"div",
            "children":[{
                "tag":"form",
                "events":{"submit":"add_item"},
                "children":[
                    {"tag":"input","attrs":{"name":"title","placeholder":"Enter..."}},
                    {"tag":"button","attrs":{"type":"submit"},"text":"Add"}
                ]
            }]
        }"""
        val node = parseDomNode(json)
        val form = node.children!![0]
        assertEquals("form", form.tag)
        assertEquals("add_item", form.event("submit"))
        assertEquals(2, form.children!!.size)
        assertEquals("title", form.children!![0].attr("name"))
    }

    @Test
    fun `cssClass returns class attribute`() {
        val node = DomNode(tag = "div", attrs = mapOf("class" to "row flex-row"))
        assertEquals("row flex-row", node.cssClass())
    }

    @Test
    fun `isRowLayout detects row classes`() {
        assertTrue(DomNode(tag = "div", attrs = mapOf("class" to "row")).isRowLayout())
        assertTrue(DomNode(tag = "div", attrs = mapOf("class" to "flex-row")).isRowLayout())
        assertTrue(DomNode(tag = "nav").isRowLayout())
        assertFalse(DomNode(tag = "div").isRowLayout())
        assertFalse(DomNode(tag = "div", attrs = mapOf("class" to "column")).isRowLayout())
    }

    @Test
    fun `unknown keys are ignored`() {
        val json = """{"tag":"div","unknownField":"whatever","text":"ok"}"""
        val node = parseDomNode(json)
        assertEquals("div", node.tag)
        assertEquals("ok", node.text)
    }

    @Test
    fun `parse real server snapshot`() {
        // Realistic snapshot from a Magnetic todo app
        val json = """{
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
        }"""
        val node = parseDomNode(json)
        assertEquals("div", node.tag)
        assertEquals("task-board", node.cssClass())
        assertEquals(4, node.children!!.size)

        // magnetic:head should parse but renderer will skip it
        assertEquals("magnetic:head", node.children!![0].tag)

        // Form
        val form = node.children!![2]
        assertEquals("add_todo", form.event("submit"))

        // List
        val list = node.children!![3]
        assertEquals("ul", list.tag)
        val todoItem = list.children!![0]
        assertEquals("todo-1", todoItem.key)
        assertEquals("toggle_1", todoItem.children!![1].event("click"))
        assertEquals("delete_1", todoItem.children!![2].event("click"))
    }
}
