import { useMemo } from "react"
import { Tag } from "rsuite";
import { ITask } from "../services/api"

type TaskTagProps = {
    task: ITask,
    size: "sm" | "md" | "lg"
}

const TaskTag = (props: TaskTagProps) => {

    const {
        task,
        size
    } = props;

    const [tagContent, tagColor] = useMemo(() => {
        const tagContent = task.status.result ?? task.status.state

        let tagColor = null
        let tagDisplay = null
        switch (tagContent) {
            case "ERROR": {
                tagColor = "red";
                tagDisplay = "Error";
                break;
            }
            case "SUCCESS": {
                tagColor = "green";
                tagDisplay = "Success";
                break;
            }
            case "RUNNING": {
                tagColor = "violet";
                tagDisplay = "Running";
                break;
            }
            case "NEW": {
                tagColor = "cyan";
                tagDisplay = "New";
                break;
            }
            default: {
                tagColor = "";
                tagDisplay = "-"
            }
        }

        return [tagDisplay, tagColor]

    }, [task])

    return ( 
        <Tag color={tagColor} size={size}>{tagContent}</Tag>
    )

}

export {
    TaskTag,
}