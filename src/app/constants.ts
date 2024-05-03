import deviceToImageArr from '../assets/only_images_and_models.json'

export const deviceToImageMap = new Map(
  deviceToImageArr.map((item) => [item.model, item.img])
)

export const countries = [
  ['cn', 'China'],
  ['ru', 'Russia'],
  ['us', 'USA'],
  ['tw', 'Taiwan'],
  ['sg', 'Singapore'],
  ['de', 'Germany'],
] as const
export const countryCodeToName = new Map(countries)
