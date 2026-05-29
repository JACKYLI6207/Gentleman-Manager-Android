import type { ImgInImgList } from './api'

export function getReaderPages(imgList: ImgInImgList[]): ImgInImgList[] {
  return imgList.filter((img) => !img.url.endsWith('shoucang.jpg'))
}
