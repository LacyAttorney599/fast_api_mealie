from pydantic_settings import BaseSettings


class Settings(BaseSettings):
    mealie_base_url: str = "http://mealie:9000"
    mealie_api_token: str = ""

    ollama_base_url: str = "http://ollama:11434"
    ollama_model: str = "qwen2.5:3b"

    tesseract_lang: str = "fra"

    class Config:
        env_file = ".env"
        extra = "ignore"


settings = Settings()
