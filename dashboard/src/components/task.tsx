import { Badge, Panel, Stack, Tag, Grid, Col, IconButton, Timeline } from "rsuite";
import { ITask } from "../services/api";
import { Terminal } from "./xterm";

import * as s from "./task.module.css";
import { useMemo } from "react";
import { TaskTag } from "./tag";

import { Icon } from '@rsuite/icons';
import { FaSvgIcon } from "./icons";
import * as faPen from '@fortawesome/free-solid-svg-icons/faPen';
import * as faCalendarPlus from '@fortawesome/free-solid-svg-icons/faCalendarPlus';
import * as faCalendarCheck from '@fortawesome/free-solid-svg-icons/faCalendarCheck';
import * as faBusinessTime from '@fortawesome/free-solid-svg-icons/faBusinessTime';
import * as faCarBurst from '@fortawesome/free-solid-svg-icons/faCarBurst';
import * as faStop from '@fortawesome/free-solid-svg-icons/faStop';
import * as faAnglesUp from '@fortawesome/free-solid-svg-icons/faAnglesUp';

type TaskProps = {
    task: ITask
}

const Task = (props: TaskProps) => {
    const { task } = props;

    return (
        <Panel shaded bordered className={s.Panel}>
            <Stack justifyContent="space-between" spacing={20} className={s.TitleStack}>
                { task.parent ? (
                    <Timeline>
                        <Timeline.Item><h4>{task.display_name}</h4></Timeline.Item>
                    </Timeline>
		) : <h4>{task.display_name}</h4> }

                <Stack spacing={8}>
                    {
                        task.locks.map(lock => {
                            return (
                                <Badge color={lock.type === "WRITE" ? "red" : "green"} content={lock.type === "WRITE" ? "W" : "R"}>
                                    <Tag size="lg">{lock.object}</Tag>
                                </Badge>
                            )
                        })
                    }
                    <TaskTag size="lg" task={task}/>
                </Stack>
            </Stack>
	    <Stack spacing={8}>
		{task.author ? <Tag color="grey"><Icon as={FaSvgIcon} faIcon={faPen} /> {task.author}{ task.creator ? " via " + task.creator : null }</Tag> : null}
		{task.created ? <Tag color="grey"><Icon as={FaSvgIcon} faIcon={faCalendarPlus} /> Created {new Date(task.created).toLocaleString()}</Tag> : null }
		{task.started ? <Tag color="grey"><Icon as={FaSvgIcon} faIcon={faBusinessTime} /> Started {new Date(task.started).toLocaleString()}</Tag> : null }
		{task.closed ? <Tag color="grey">
		    { task.status == "SUCCESS" ?
			<Icon as={FaSvgIcon} faIcon={faCalendarCheck} />+" Finished"
		    : task.status == "FAILED" ?
			<Icon as={FaSvgIcon} faIcon={faCarBurst} />+" Failed"
		    :
			<Icon as={FaSvgIcon} faIcon={faStop} />+" Closed"
		    }
		    {new Date(task.created).toLocaleString()
		}</Tag> : null }
	    </Stack>
	    <Stack spacing={8} style={{ marginTop: ".5em" }}>
                {task.parent ? <IconButton color="green" appearance="primary" size="xs" key={task.parent} href={`/tasks/${task.parent}`} placement="left" icon={<Icon as={FaSvgIcon} faIcon={faAnglesUp} />}>View Parent</IconButton> : null}
	    </Stack>
            <Terminal key={task.id} taskId={task.id} />
        </Panel>
    )
}

export {
    Task
}

