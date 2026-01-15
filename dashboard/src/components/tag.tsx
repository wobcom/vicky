import {useMemo} from "react"
import {Tag} from "rsuite";
import {ITask} from "../services/api"
import {FilterOption} from "./tasks";

type TaskTagProps = {
    task: ITask,
    size: "sm" | "md" | "lg"
    options: FilterOption[],
}
const fallbackTag: FilterOption = { color: "", label: "-", value: "None" }

const TaskTag = (props: TaskTagProps) => {

    const {
        task,
        size,
        options,
    } = props;

    const tag: FilterOption = useMemo(() => {
        const tagContent = task.status.state + (task.status.result ? "::" + task.status.result : "")

        return options.find(o => o.value == tagContent) ?? fallbackTag

    }, [task.status])

    return (
        <Tag
            style={{
                width: "6em",
                display: "inline-flex",
                alignItems: "center",
                justifyContent: "center",
                textAlign: "center",
            }}
            color={tag.color}
            size={size}
        >
            {tag.label}
        </Tag>
    )

}

export {
    TaskTag,
}
