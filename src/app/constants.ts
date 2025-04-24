import deviceToImageArr from '../assets/only_images_and_models.json'

export const deviceToImageMap = new Map(
  deviceToImageArr.map((item) => [item.model, item.img])
)
