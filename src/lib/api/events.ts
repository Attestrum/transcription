/**
 * Typed wrappers over listen(). Each returns the unlisten function; callers
 * filter by jobId / modelId where relevant.
 */

import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type {
	AppError,
	DoneEvent,
	DownloadProgressEvent,
	JobErrorEvent,
	LevelUpdate,
	ProgressEvent,
	SegmentEvent
} from './types';

const on =
	<T>(name: string) =>
	(handler: (payload: T) => void): Promise<UnlistenFn> =>
		listen<T>(name, (e) => handler(e.payload));

export const onDownloadProgress = on<DownloadProgressEvent>('model:download:progress');
export const onDownloadDone = on<string>('model:download:done');
export const onDownloadError = on<[string, AppError]>('model:download:error');

export const onRecordLevel = on<LevelUpdate>('record:level');

export const onTranscribeSegment = on<SegmentEvent>('transcribe:segment');
export const onTranscribeProgress = on<ProgressEvent>('transcribe:progress');
export const onTranscribeDone = on<DoneEvent>('transcribe:done');
export const onTranscribeCancelled = on<number>('transcribe:cancelled');
export const onTranscribeError = on<JobErrorEvent>('transcribe:error');
